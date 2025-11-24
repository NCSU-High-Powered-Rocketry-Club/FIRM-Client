from ._firm_client import FIRMPacket, PyFIRMParser
import threading
import serial
import time


class FIRM:
    def __init__(self, port: str, baudrate: int = 115_200):
        self._parser = PyFIRMParser()
        self._serial_port = serial.Serial(port, baudrate)
        self._serial_reader_thread = None
        self._stop_event = threading.Event()

    def __enter__(self):
        """Context manager entry: initialize the parser."""
        self.initialize()
        return self

    def __exit__(self, exc_type, exc_value, traceback):
        """Context manager exit: Ensure serial port is closed on exit."""
        self.close()

    def initialize(self):
        """Open serial and prepare for parsing packets by spawning a new thread."""
        if not self._serial_port.is_open:
            self._serial_port.open()
        self._stop_event.clear()
        if (
            self._serial_reader_thread is None
            or not self._serial_reader_thread.is_alive()
        ):
            self._serial_reader_thread = threading.Thread(
                target=self._serial_reader, name="Packet-Reader-Thread", daemon=True
            )
            self._serial_reader_thread.start()

    def close(self):
        """Close the serial port, and stop the packet reader thread."""
        self._stop_event.set()
        if self._serial_reader_thread is not None:
            self._serial_reader_thread.join(timeout=1.0)
            self._serial_reader_thread = None
        if self._serial_port.is_open:
            self._serial_port.close()

    def get_number_of_available_packets(self) -> int:
        """
        Get the number of FIRMPacket objects currently available from the
        underlying parser.

        Note: this will consume available packets from the parser. To avoid
        losing packets, call `get_data_packets` which returns the packets.
        """
        # Drain available packets and return the count (consumes them).
        packets = self.get_data_packets(block=False)
        return len(packets)

    def get_most_recent_data_packet(self) -> FIRMPacket:
        """
        Retrieve the most recent FIRMPacket object parsed.
        """
        return self.get_data_packets(block=True)[-1]

    def get_data_packets(
        self,
        block: bool = True,
    ) -> list[FIRMPacket]:
        """
        Retrieve FIRMPacket objects parsed by the background thread by
        calling into the parser's `get_packet` method.

        Args:
            block: If True, wait for at least one packet.

        Returns:
            List of FIRMPacket objects. Calling this consumes the packets
            from the parser's internal queue.
        """
        firm_packets: list[FIRMPacket] = []

        # If blocking, wait until at least one packet is available.
        if block:
            while True:
                pkt = self._parser.get_packet()
                if pkt is None:
                    if firm_packets:
                        break
                    # Yeild
                    time.sleep(0)
                    continue
                firm_packets.append(pkt)

        # Drain any remaining available packets without blocking.
        while True:
            pkt = self._parser.get_packet()
            if pkt is None:
                break
            firm_packets.append(pkt)

        return firm_packets

    def _serial_reader(self):
        """Continuously read from serial port, parse packets, and enqueue them."""
        while not self._stop_event.is_set():
            new_bytes = self._serial_port.read(self._serial_port.in_waiting)
            # Parse as many packets as possible
            self._parser.parse_bytes(new_bytes)
