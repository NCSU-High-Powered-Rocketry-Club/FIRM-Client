import firm_client
import time

client = firm_client.FIRMClient("COM12")
client.start()

print("Please rotate the device in all directions...")

# This line will block for 15 seconds
result = client.run_and_apply_magnetometer_calibration(
    collection_duration_seconds=60.0, 
    apply_timeout_seconds=1.0
)

if result is True:
    print("Calibration Success! Applied to device.")
elif result is False:
    print("Calibration calculated, but device rejected the update.")
elif result is None:
    print("Calibration Failed: Not enough data points.")
else:
    print("Error occurred.")

client.stop()