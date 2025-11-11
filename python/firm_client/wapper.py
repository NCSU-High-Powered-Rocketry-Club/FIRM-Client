from . import _firm_client

import threading
import serial


class FIRM:
    def __init__(self, port: str, baudrate: int = 115_200):
        self._parser = _firm_client.PythonSerialParser()
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
        Get the number of FIRMPacket objects currently available in the queue.

        Returns:
            Number of available FIRMPacket objects.
        """
        return self._packet_queue.qsize()

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
        Retrieve FIRMPacket objects parsed by the background thread.

        Args:
            block: If True, wait for at least one packet.

        Returns:
            List of FIRMPacket objects.
        """
        firm_packets: list[FIRMPacket] = []

        if block:
            # Keep waiting until we successfully get a packet
            while not firm_packets:
                packet = self._packet_queue.get()
                firm_packets.append(packet)

        while self._packet_queue.qsize() > 0:
            firm_packets.append(self._packet_queue.get_nowait())

        return firm_packets
