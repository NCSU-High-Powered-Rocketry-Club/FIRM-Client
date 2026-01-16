import firm_client

PORT = "COM8"
BAUD = 2_000_000
TIMEOUT = 0.1
RESPONSE_TIMEOUT = 5.0

client = firm_client.FIRMClient(PORT, BAUD, TIMEOUT)
client.start()

device_info = client.get_device_info(timeout_seconds=RESPONSE_TIMEOUT)
if device_info:
    print(f"Version: {device_info.firmware_version}, Id: {device_info.id}, ")

print(
    client.set_device_config(
        "name", 102, firm_client.DeviceProtocol.UART, timeout_seconds=RESPONSE_TIMEOUT
    )
)

device_config = client.get_device_config(timeout_seconds=RESPONSE_TIMEOUT)
if device_config:
    print(
        f"Name: {device_config.name}, Frequency: {device_config.frequency}, Protocol: {device_config.protocol}"
    )
