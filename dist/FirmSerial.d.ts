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
export declare class FirmSerial {
    private firm;
    private reader;
    private running;
    private packetQueue;
    private packetWaiters;
    private constructor();
    /**
     * Connect to a serial device and start the background read loop.
     *
     * Usage:
     *   const dev = await FirmSerial.connect({ baudRate: 115200 });
     */
    static connect(options?: FirmConnectOptions): Promise<FirmSerial>;
    /**
     * Internal read loop:
     * - reads chunks from Web Serial
     * - feeds them into the Rust parser
     * - drains parsed packets into a queue
     */
    private startReadLoop;
    private enqueuePacket;
    private flushWaitersWithNull;
    /**
     * Get a single packet.
     * - If queue is non-empty, returns immediately.
     * - Otherwise waits until a packet arrives or the stream ends.
     *
     * Returns null if no more packets will ever arrive.
     */
    getPacket(): Promise<FirmPacket | null>;
    /**
     * Async iterator of packets:
     *
     *   for await (const pkt of dev.packets()) {
     *     console.log(pkt);
     *   }
     */
    packets(): AsyncGenerator<FirmPacket, void>;
    /**
     * Stop reading and clean up.
     */
    close(): Promise<void>;
}
