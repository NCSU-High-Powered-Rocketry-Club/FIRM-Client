import init, { JSFIRMParser, FIRMPacket } from '../../pkg/firm_client.js';

/**
 * Data packet received from FIRM.
 *
 * This is the Rust `FIRMPacket` type exported via wasm-bindgen.
 */
export { FIRMPacket };

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
export class FIRM {
  /** Underlying WASM-backed streaming parser. */
  private wasm: JSFIRMParser;

  /** Reader for the Web Serial stream. */
  private reader: ReadableStreamDefaultReader<Uint8Array> | null = null;

  /** Whether the background read loop is currently running. */
  private running = false;

  /** Queue of parsed packets waiting to be consumed. */
  private packetQueue: FIRMPacket[] = [];

  /** Waiters that are pending the next available packet. */
  private packetWaiters: Array<(pkt: FIRMPacket | null) => void> = [];

  private constructor(wasm: JSFIRMParser) {
    this.wasm = wasm;
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
  static async connect(options: FIRMConnectOptions = {}): Promise<FIRM> {
    if (!('serial' in navigator)) {
      throw new Error('Web Serial API not available in this browser');
    }

    // Initialize the WASM module.
    await init();

    const wasm = new JSFIRMParser();
    const baudRate = options.baudRate ?? 115200;

    // Ask user for a serial device & open it.
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const port = await (navigator as any).serial.requestPort();
    await port.open({ baudRate });

    const reader: ReadableStreamDefaultReader<Uint8Array> = port.readable.getReader();

    const firm = new FIRM(wasm);
    firm.reader = reader;
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
          // Push raw bytes into the Rust/WASM parser.
          this.wasm.parse_bytes(value);

          // Drain all packets that are ready.
          while (true) {
            const pkt = this.wasm.get_packet(); // FIRMPacket | undefined
            if (!pkt) break; // `None` -> `undefined`
            this.enqueuePacket(pkt);
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
    }
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
    this.flushWaitersWithNull();
  }
}
