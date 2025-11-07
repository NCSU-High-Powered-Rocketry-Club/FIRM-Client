// ts/FirmSerial.ts

// This imports the wasm-bindgen JS glue generated into ../pkg by wasm-pack
// after you run the wasm build step.
import init, { FIRM } from "../pkg/firm_client";

// You can refine this later once you know FIRMPacket’s shape
export type FirmPacket = any;

export interface FirmConnectOptions {
  baudRate?: number;
}

/**
 * High-level browser API for the FIRM parser:
 * - handles Web Serial
 * - streams data into the Rust parser
 * - exposes async packet methods
 */
export class FirmSerial {
  private firm: FIRM;
  private reader: ReadableStreamDefaultReader<Uint8Array> | null = null;
  private running = false;

  private packetQueue: FirmPacket[] = [];
  private packetWaiters: Array<(pkt: FirmPacket | null) => void> = [];

  private constructor(firm: FIRM) {
    this.firm = firm;
  }

  /**
   * Connect to a serial device and start the background read loop.
   *
   * Usage:
   *   const dev = await FirmSerial.connect({ baudRate: 115200 });
   */
  static async connect(options: FirmConnectOptions = {}): Promise<FirmSerial> {
    if (!("serial" in navigator)) {
      throw new Error("Web Serial API not available in this browser");
    }

    // Initialize the wasm module (from ../pkg/firm_client.js)
    await init();

    const firm = new FIRM();
    const baudRate = options.baudRate ?? 115200;

    // Ask user for a serial device & open it
    const port = await (navigator as any).serial.requestPort();
    await port.open({ baudRate });

    const reader: ReadableStreamDefaultReader<Uint8Array> =
      port.readable.getReader();

    const dev = new FirmSerial(firm);
    dev.reader = reader;
    dev.startReadLoop(); // fire-and-forget
    return dev;
  }

  /**
   * Internal read loop:
   * - reads chunks from Web Serial
   * - feeds them into the Rust parser
   * - drains parsed packets into a queue
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
          // Push raw bytes into Rust parser
          this.firm.parse_bytes(value);

          // Drain all packets that are ready
          while (true) {
            const pkt = this.firm.get_packet();
            if (pkt === null) break;
            this.enqueuePacket(pkt as FirmPacket);
          }
        }
      }
    } catch (err) {
      console.error("[FirmSerial] read loop error:", err);
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

  private enqueuePacket(pkt: FirmPacket) {
    if (this.packetWaiters.length > 0) {
      const waiter = this.packetWaiters.shift()!;
      waiter(pkt);
    } else {
      this.packetQueue.push(pkt);
    }
  }

  private flushWaitersWithNull() {
    while (this.packetWaiters.length > 0) {
      const waiter = this.packetWaiters.shift()!;
      waiter(null);
    }
  }

  /**
   * Get a single packet.
   * - If queue is non-empty, returns immediately.
   * - Otherwise waits until a packet arrives or the stream ends.
   *
   * Returns null if no more packets will ever arrive.
   */
  async getPacket(): Promise<FirmPacket | null> {
    if (this.packetQueue.length > 0) {
      return this.packetQueue.shift()!;
    }

    if (!this.running) {
      return null;
    }

    return new Promise<FirmPacket | null>((resolve) => {
      this.packetWaiters.push(resolve);
    });
  }

  /**
   * Async iterator of packets:
   *
   *   for await (const pkt of dev.packets()) {
   *     console.log(pkt);
   *   }
   */
  async *packets(): AsyncGenerator<FirmPacket, void> {
    while (true) {
      const pkt = await this.getPacket();
      if (pkt === null) return;
      yield pkt;
    }
  }

  /**
   * Stop reading and clean up.
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
