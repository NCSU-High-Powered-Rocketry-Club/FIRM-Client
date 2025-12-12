import time
from firm_client import FIRM, FIRMPacket

def main():
    # Replace 'COM3' with your actual serial port (e.g., '/dev/ttyUSB0' on Linux)
    PORT = "COM5" 
    BAUDRATE = 115200

    print(f"Connecting to FIRM device on {PORT} at {BAUDRATE} baud...")

    try:
        with FIRM(PORT, BAUDRATE) as firm:
            print("Connected!")
            start_time = time.time()
            while time.time() - start_time < 5:
                # Get all available packets (non-blocking)
                packets = firm.get_data_packets(block=False)
                
                for pkt in packets:
                    print(f"Received Packet: Time={pkt.timestamp_seconds:.2f}, "
                          f"AccelZ={pkt.accel_z_meters_per_s2:.2f} m/s^2")
                
                time.sleep(0.1)
            
            print("Done.")

    except Exception as e:
        print(f"Error: {e}")
        print("Note: Ensure the device is connected and the PORT variable is correct.")

if __name__ == "__main__":
    main()
