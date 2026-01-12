import init, { FIRMDataParser, FIRMCommandBuilder } from '../../pkg/firm_client.js';
import { FIRMPacket, FIRMResponse, DeviceInfo, DeviceConfig, DeviceProtocol } from './types.js';

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
  private rawBytesListeners: ((bytes: Uint8Array) => void)[] = [];

  /** Reader for the Web Serial stream. */
  private reader: ReadableStreamDefaultReader<Uint8Array> | null = null;

  /** Writer for the Web Serial stream. */
  private writer: WritableStreamDefaultWriter<Uint8Array> | null = null;

  /** Whether the background read loop is currently running. */
  private running = false;

  private packetQueue: FIRMPacket[] = [];
  private packetWaiters: ((pkt: FIRMPacket | null) => void)[] = [];
  private responseQueue: FIRMResponse[] = [];
  private responseWaiters: ((res: FIRMResponse) => void)[] = [];

  private closed = false;

  private constructor(wasm: FIRMDataParser) {
    this.dataParser = wasm;
  }

  /**
   * Connects to a serial device and starts the background read loop.
   *
   * @param options Connection options (e.g. baudRate).
   * @returns A connected FIRM instance.
   */
  static async connect(options: FIRMConnectOptions = {}): Promise<FIRMClient> {
    if (!('serial' in navigator)) throw new Error('Web Serial API not available');
    await init();
    const dataParser = new FIRMDataParser();
    const baudRate = options.baudRate ?? 115200;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const port = await (navigator as any).serial.requestPort();
    await port.open({ baudRate });
    const reader = port.readable.getReader();
    const writer = port.writable.getWriter();
    const firm = new FIRMClient(dataParser);
    firm.port = port;
    firm.reader = reader;
    firm.writer = writer;
    firm
      .startReadLoop()
      .catch((err) => console.warn('[FIRM] read loop task failed (ignored):', err));
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
        if (done || !this.running) break;
        if (value?.length) {
          this.rawBytesListeners.forEach((fn) => {
            try {
              fn(value);
            } catch (e) {
              console.warn('[FIRM] raw bytes listener error:', e);
            }
          });
          this.dataParser.parse_bytes(value);
          let pkt;
          while ((pkt = this.dataParser.get_packet() as FIRMPacket | null)) this.enqueuePacket(pkt);
          let res;
          while ((res = this.dataParser.get_response() as FIRMResponse | null))
            this.enqueueResponse(res);
        }
      }
    } catch (err) {
      console.warn('[FIRM] read loop ended:', err);
      this.flushWaitersWithNull();
    } finally {
      this.running = false;
      this.flushWaitersWithNull();
      for (const [obj, fn] of [
        [this.reader, 'cancel'],
        [this.reader, 'releaseLock'],
        [this.writer, 'close'],
        [this.writer, 'releaseLock'],
        [this.port, 'close'],
      ]) {
        try {
          obj &&
            typeof obj[fn] === 'function' &&
            (fn === 'close' || fn === 'cancel' ? await obj[fn]() : obj[fn]());
        } catch {}
      }
      this.reader = this.writer = this.port = null;
    }
  }

  /**
   * Sends raw bytes to the device.
   * @param bytes The bytes to send.
   */
  async sendBytes(bytes: Uint8Array): Promise<void> {
    if (!this.writer) throw new Error('Writer not available');
    await this.writer.write(bytes);
  }

  /**
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
  private async sendAndWait<T>(
    buildCmd: () => Uint8Array,
    matcher: (res: FIRMResponse) => T | undefined,
    timeout = RESPONSE_TIMEOUT_MS,
  ): Promise<T | null> {
    await this.sendBytes(buildCmd());
    try {
      return await this.waitForResponse(matcher, timeout);
    } catch {
      return null;
    }
  }

  /**
   * Gets device info from the FIRM device.
   * @returns The device info, or null if the request timed out.
   */
  async getDeviceInfo(): Promise<DeviceInfo | null> {
    return this.sendAndWait(
      () => FIRMCommandBuilder.build_get_device_info(),
      (res) => ('GetDeviceInfo' in res ? res.GetDeviceInfo : undefined),
    );
  }

  /**
   * Gets device configuration from the FIRM device.
   * @returns The device configuration, or null if the request timed out.
   */
  async getDeviceConfig(): Promise<DeviceConfig | null> {
    return this.sendAndWait(
      () => FIRMCommandBuilder.build_get_device_config(),
      (res) => ('GetDeviceConfig' in res ? res.GetDeviceConfig : undefined),
    );
  }

  /**
   * Sets device configuration on the FIRM device.
   * @param name the device name, 32 characters max.
   * @param frequency the data frequency in Hz, 1-1000 Hz.
   * @param protocol the communication protocol.
   * @returns True if the configuration was set successfully, false otherwise.
   */
  async setDeviceConfig(
    name: string,
    frequency: number,
    protocol: DeviceProtocol,
  ): Promise<boolean> {
    return (
      (await this.sendAndWait(
        () => FIRMCommandBuilder.build_set_device_config(name, frequency, protocol),
        (res) => ('SetDeviceConfig' in res ? res.SetDeviceConfig : undefined),
      )) ?? false
    );
  }

  /**
   * Sends a cancel command to the device (e.g., to abort a calibration).
   * @returns True if acknowledged, false if timeout or not acknowledged.
   */
  async sendCancelCommand(): Promise<boolean> {
    return (
      (await this.sendAndWait(
        () => FIRMCommandBuilder.build_cancel(),
        (res) => ('Cancel' in res ? res.Cancel : undefined),
      )) ?? false
    );
  }

  /**
   * Sends a reboot command to the device.
   */
  async reboot(): Promise<void> {
    await this.sendBytes(FIRMCommandBuilder.build_reboot());
  }

  /**
   * Enqueues a newly parsed packet or resolves a pending waiter.
   *
   * @param dataPacket Parsed FIRM data packet.
   */
  private enqueuePacket(dataPacket: FIRMPacket): void {
    (this.packetWaiters.shift() || this.packetQueue.push.bind(this.packetQueue))(dataPacket);
  }

  /**
   * Enqueues a newly parsed response and notifies waiters.
   *
   * @param response Parsed FIRM response.
   */
  private enqueueResponse(response: FIRMResponse): void {
    this.responseQueue.push(response);
    this.responseWaiters.forEach((waiter) => waiter(response));
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
    while (this.packetWaiters.length)
      (this.packetWaiters.shift() as (pkt: FIRMPacket | null) => void)(null);
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
    try {
      if (this.closed) return;
      this.closed = true;

      this.running = false;
      if (this.reader) {
        try {
          await this.reader.cancel();
        } catch {
          // ignore
        }
        try {
          this.reader.releaseLock();
        } catch {
          // ignore
        }
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

        this.reader = null;
        this.writer = null;
        this.port = null;
      }

      this.reader = null;
      this.writer = null;
      this.port = null;
      this.flushWaitersWithNull();
    } catch (e) {
      // Swallow any unexpected error so close() never rejects
      // (prevents unhandled promise rejection on reboot/disconnect)
      console.warn('[FIRM] close() failed (swallowed):', e);
      // Re-throw for debug if needed:
      // throw e;
    }
  }
}
