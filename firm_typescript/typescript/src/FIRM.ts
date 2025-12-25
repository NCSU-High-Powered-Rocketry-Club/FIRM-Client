import init, { FIRMDataParser, FIRMCommandBuilder } from '../../pkg/firm_client.js';
import { FIRMPacket, FIRMResponse, DeviceInfo, DeviceConfig, CalibrationStatus } from './types.js';

/**
 * Data packet received from FIRM.
 *
 * This is the Rust `FIRMPacket` type exported via wasm-bindgen.
 */
export { FIRMPacket, FIRMResponse };

const RESPONSE_TIMEOUT_MS = 5000;
const CALIBRATION_TIMEOUT_MS = 20000;

/** Options for connecting to a FIRM device over Web Serial. */
export interface FIRMConnectOptions {
  /** Serial baud rate (default: 115200). */
  baudRate?: number;
}

/**
 * High-level browser API for the FIRM parser.
 *
 * - Handles Web Serial connection.
 * - Streams raw bytes into the Rust parser (wasm).
 * - Exposes methods to read parsed telemetry packets.
 */
export class FIRMClient {
  /** Underlying WASM-backed streaming parser. */
  private dataParser: FIRMDataParser;

  /** Serial port (Web Serial API). Kept so we can close/reconnect cleanly. */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  private port: any | null = null;

  /** Subscribers for raw incoming serial bytes. */
  private rawBytesListeners: Array<(bytes: Uint8Array) => void> = [];

  /** Reader for the Web Serial stream. */
  private reader: ReadableStreamDefaultReader<Uint8Array> | null = null;

  /** Writer for the Web Serial stream. */
  private writer: WritableStreamDefaultWriter<Uint8Array> | null = null;

  /** Whether the background read loop is currently running. */
  private running = false;

  /** Queue of parsed packets waiting to be consumed. */
  private packetQueue: FIRMPacket[] = [];

  /** Waiters that are pending the next available packet. */
  private packetWaiters: Array<(pkt: FIRMPacket | null) => void> = [];

  /** Queue of parsed responses waiting to be consumed. */
  private responseQueue: FIRMResponse[] = [];

  /** Waiters that are pending a specific response. */
  private responseWaiters: Array<(res: FIRMResponse) => void> = [];

  private constructor(wasm: FIRMDataParser) {
    this.dataParser = wasm;
  }

  /**
   * Connects to a serial device and starts the background read loop.
   *
   * @param options Connection options (e.g. baudRate).
   * @returns A connected FIRM instance.
   *
   * Usage:
   *   const firm = await FIRM.connect({ baudRate: 115200 });
   */
  static async connect(options: FIRMConnectOptions = {}): Promise<FIRMClient> {
    if (!('serial' in navigator)) {
      throw new Error('Web Serial API not available in this browser');
    }

    // Initialize the WASM module.
    await init();

    const dataParser = new FIRMDataParser();
    const baudRate = options.baudRate ?? 115200;

    // Ask user for a serial device & open it.
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const port = await (navigator as any).serial.requestPort();
    await port.open({ baudRate });

    const reader: ReadableStreamDefaultReader<Uint8Array> = port.readable.getReader();
    const writer: WritableStreamDefaultWriter<Uint8Array> = port.writable.getWriter();

    const firm = new FIRMClient(dataParser);
    firm.port = port;
    firm.reader = reader;
    firm.writer = writer;
    firm.startReadLoop();
    return firm;
  }

  /**
   * Internal read loop that continuously reads from Web Serial,
   * feeds raw bytes into the WASM parser, and enqueues parsed packets.
   */
  private async startReadLoop(): Promise<void> {
    if (!this.reader) return;

    this.running = true;

    try {
      while (this.running) {
        const { value, done } = await this.reader.read();
        if (done || !this.running) {
          break;
        }

        if (value && value.length > 0) {
          // Notify listeners of raw incoming bytes.
          if (this.rawBytesListeners.length > 0) {
            for (const listener of this.rawBytesListeners) {
              try {
                listener(value);
              } catch (e) {
                console.warn('[FIRM] raw bytes listener error:', e);
              }
            }
          }

          // Push raw bytes into the Rust/WASM parser.
          this.dataParser.parse_bytes(value);

          // Drain all packets that are ready.
          while (true) {
            const pkt = this.dataParser.get_packet() as FIRMPacket | null; // FIRMPacket | null
            if (!pkt) break; // `None` -> `null`
            this.enqueuePacket(pkt);
          }

          // Drain all responses that are ready.
          while (true) {
            const res = this.dataParser.get_response() as FIRMResponse | null;
            if (!res) break;
            this.enqueueResponse(res);
          }
        }
      }
    } catch (err) {
      console.error('[FIRM] read loop error:', err);
      this.flushWaitersWithNull();
    } finally {
      this.running = false;
      try {
        await this.reader?.cancel();
      } catch {
        // ignore
      }
      this.reader?.releaseLock();

      try {
        await this.writer?.close();
      } catch {
        // ignore
      }
      try {
        this.writer?.releaseLock();
      } catch {
        // ignore
      }

      try {
        await this.port?.close();
      } catch {
        // ignore
      }
    }
  }

  /**
   * Sends raw bytes to the device.
   * @param bytes The bytes to send.
   */
  async sendBytes(bytes: Uint8Array): Promise<void> {
    if (!this.writer) {
      throw new Error('Writer not available');
    }
    await this.writer.write(bytes);
  }

  /**
   * Subscribes to raw incoming serial bytes from the device.
   *
   * @param listener Callback invoked with each incoming chunk.
   * @returns Unsubscribe function.
   */
  onRawBytes(listener: (bytes: Uint8Array) => void): () => void {
    this.rawBytesListeners.push(listener);
    return () => {
      const idx = this.rawBytesListeners.indexOf(listener);
      if (idx !== -1) {
        this.rawBytesListeners.splice(idx, 1);
      }
    };
  }

  /**
   * Sends a command to get device info.
   * @returns The device info, or null if the request timed out.
   */
  async getDeviceInfo(): Promise<DeviceInfo | null> {
    const bytes = FIRMCommandBuilder.build_get_device_info();
    await this.sendBytes(bytes);

    try {
      return await this.waitForResponse(
        (res) => ('GetDeviceInfo' in res ? res.GetDeviceInfo : undefined),
        RESPONSE_TIMEOUT_MS,
      );
    } catch (e) {
      console.warn('Timeout waiting for GetDeviceInfo response');
      return null;
    }
  }

  /**
   * Sends a command to get device configuration.
   * @returns The device configuration, or null if the request timed out.
   */
  async getDeviceConfig(): Promise<DeviceConfig | null> {
    const bytes = FIRMCommandBuilder.build_get_device_config();
    await this.sendBytes(bytes);

    try {
      return await this.waitForResponse(
        (res) => ('GetDeviceConfig' in res ? res.GetDeviceConfig : undefined),
        RESPONSE_TIMEOUT_MS,
      );
    } catch (e) {
      console.warn('Timeout waiting for GetDeviceConfig response');
      return null;
    }
  }

  /**
   * Sends a command to set device configuration.
   * @param name Device name.
   * @param frequency Frequency in Hz.
   * @param protocol Protocol ID (1=USB, 2=UART, 3=I2C, 4=SPI).
   * @returns True if the configuration was set successfully, false otherwise.
   */
  async setDeviceConfig(name: string, frequency: number, protocol: number): Promise<boolean> {
    const bytes = FIRMCommandBuilder.build_set_device_config(name, frequency, protocol);
    await this.sendBytes(bytes);

    try {
      return await this.waitForResponse(
        (res) => ('SetDeviceConfig' in res ? res.SetDeviceConfig : undefined),
        RESPONSE_TIMEOUT_MS,
      );
    } catch (e) {
      console.warn('Timeout waiting for SetDeviceConfig response');
      return false;
    }
  }

  /**
   * Sends a command to run IMU calibration.
   * @returns The calibration status, or null if the request timed out.
   */
  async runIMUCalibration(): Promise<CalibrationStatus | null> {
    const bytes = FIRMCommandBuilder.build_run_imu_calibration();
    await this.sendBytes(bytes);

    try {
      return await this.waitForResponse(
        (res) => ('RunIMUCalibration' in res ? res.RunIMUCalibration : undefined),
        CALIBRATION_TIMEOUT_MS,
      );
    } catch (e) {
      console.warn('Timeout waiting for RunIMUCalibration response');
      return null;
    }
  }

  /**
   * Sends a command to run Magnetometer calibration.
   * @returns The calibration status, or null if the request timed out.
   */
  async runMagnetometerCalibration(): Promise<CalibrationStatus | null> {
    const bytes = FIRMCommandBuilder.build_run_magnetometer_calibration();
    await this.sendBytes(bytes);

    try {
      return await this.waitForResponse(
        (res) => ('RunMagnetometerCalibration' in res ? res.RunMagnetometerCalibration : undefined),
        CALIBRATION_TIMEOUT_MS,
      );
    } catch (e) {
      console.warn('Timeout waiting for RunMagnetometerCalibration response');
      return null;
    }
  }

  /**
   * Sends a cancel command to the device (e.g., to abort a calibration).
   * @returns True if the device acknowledged the cancel, or null if the request timed out.
   */
  async sendCancelCommand(): Promise<boolean | null> {
    const bytes = FIRMCommandBuilder.build_cancel();
    await this.sendBytes(bytes);

    try {
      return await this.waitForResponse(
        (res) => ('Cancel' in res ? res.Cancel : undefined),
        RESPONSE_TIMEOUT_MS,
      );
    } catch (e) {
      console.warn('Timeout waiting for Cancel response');
      return null;
    }
  }

  /**
   * Sends a command to reboot the device.
   */
  async reboot(): Promise<void> {
    const bytes = FIRMCommandBuilder.build_reboot();
    await this.sendBytes(bytes);
  }

  /**
   * Enqueues a newly parsed packet or resolves a pending waiter.
   *
   * @param dataPacket Parsed FIRM data packet.
   */
  private enqueuePacket(dataPacket: FIRMPacket): void {
    if (this.packetWaiters.length > 0) {
      const waiter = this.packetWaiters.shift()!;
      waiter(dataPacket);
    } else {
      this.packetQueue.push(dataPacket);
    }
  }

  /**
   * Enqueues a newly parsed response and notifies waiters.
   *
   * @param response Parsed FIRM response.
   */
  private enqueueResponse(response: FIRMResponse): void {
    this.responseQueue.push(response);
    // Notify all waiters so they can check if this is the response they want.
    [...this.responseWaiters].forEach((waiter) => waiter(response));
  }

  /**
   * Waits for a specific response that matches the predicate.
   *
   * @param matcher Function that returns the desired value if the response matches, or undefined.
   * @param timeoutMs Max time to wait in milliseconds.
   */
  private async waitForResponse<T>(
    matcher: (res: FIRMResponse) => T | undefined,
    timeoutMs: number,
  ): Promise<T> {
    // 1. Check existing queue
    for (let i = 0; i < this.responseQueue.length; i++) {
      const match = matcher(this.responseQueue[i]);
      if (match !== undefined) {
        this.responseQueue.splice(i, 1); // Consume it
        return match;
      }
    }

    // 2. Wait for new responses
    return new Promise<T>((resolve, reject) => {
      const timeoutId = setTimeout(() => {
        cleanup();
        reject(new Error('Timeout waiting for response'));
      }, timeoutMs);

      const onResponse = (res: FIRMResponse) => {
        const match = matcher(res);
        if (match !== undefined) {
          // We found it! Remove from queue.
          const idx = this.responseQueue.indexOf(res);
          if (idx !== -1) {
            this.responseQueue.splice(idx, 1);
          }
          cleanup();
          resolve(match);
        }
      };

      const cleanup = () => {
        clearTimeout(timeoutId);
        const idx = this.responseWaiters.indexOf(onResponse);
        if (idx !== -1) {
          this.responseWaiters.splice(idx, 1);
        }
      };

      this.responseWaiters.push(onResponse);
    });
  }

  /**
   * Resolves all pending waiters with null (used on shutdown/error).
   */
  private flushWaitersWithNull(): void {
    while (this.packetWaiters.length > 0) {
      const waiter = this.packetWaiters.shift()!;
      waiter(null);
    }
  }

  /**
   * Internal helper that waits for the next packet in FIFO order.
   *
   * @returns Next packet, or null if the stream has ended.
   */
  private async waitForNextPacket(): Promise<FIRMPacket | null> {
    if (this.packetQueue.length > 0) {
      return this.packetQueue.shift()!;
    }

    if (!this.running) {
      return null;
    }

    return new Promise<FIRMPacket | null>((resolve) => {
      this.packetWaiters.push(resolve);
    });
  }

  /**
   * Returns the most recent packet, discarding any older queued packets.
   *
   * If no packets are queued, waits for the next one to arrive.
   * Returns null if the stream has ended and no packets remain.
   *
   * @returns The latest FIRMPacket, or null if no more packets will arrive.
   */
  async getMostRecentDataPacket(): Promise<FIRMPacket | null> {
    if (!this.running && this.packetQueue.length === 0) {
      return null;
    }

    if (this.packetQueue.length > 0) {
      const latest = this.packetQueue[this.packetQueue.length - 1];
      this.packetQueue = [];
      return latest;
    }

    return this.waitForNextPacket();
  }

  /**
   * Async iterator over packets in arrival order.
   *
   * Usage:
   *   for await (const pkt of firm.getDataPackets()) {
   *     console.log(pkt.timestamp_seconds);
   *   }
   *
   * @returns Async generator yielding FIRMPacket objects.
   */
  async *getDataPackets(): AsyncGenerator<FIRMPacket, void> {
    while (true) {
      const pkt = await this.waitForNextPacket();
      if (pkt === null) return;
      yield pkt;
    }
  }

  /**
   * Stops reading from the serial device and clears pending waiters.
   *
   * @returns Resolves when the reader has been cancelled and released.
   */
  async close(): Promise<void> {
    this.running = false;
    if (this.reader) {
      try {
        await this.reader.cancel();
      } catch {
        // ignore
      }
      this.reader.releaseLock();
    }

    if (this.writer) {
      try {
        await this.writer.close();
      } catch {
        // ignore
      }
      try {
        this.writer.releaseLock();
      } catch {
        // ignore
      }
    }

    if (this.port) {
      try {
        await this.port.close();
      } catch {
        // ignore
      }
    }

    this.reader = null;
    this.writer = null;
    this.port = null;
    this.flushWaitersWithNull();
  }
}
