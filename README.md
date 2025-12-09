# FIRM-Client

A modular Rust library for parsing FIRM data packets, with bindings for Python and WebAssembly.

## Project Structure

The project is organized as a Cargo workspace with the following crates:

- **`firm_core`**: The core `no_std` crate containing the packet parser, CRC logic, and data structures. This is the foundation for all other crates and can be used in embedded environments.
- **`firm_rust`**: A high-level Rust API that uses `serialport` to read from a serial device and provides a threaded client for receiving packets.
- **`firm_py`**: Python bindings for the Rust client.
- **`firm_wasm`**: WebAssembly bindings and TypeScript code for using the parser in web applications.

## Philosophy

The goal of FIRM-Client is to provide a single, efficient, and correct implementation of the FIRM protocol that can be used across different ecosystems (Rust, Python, Web/JS, Embedded). By centralizing the parsing logic in `firm_core`, we ensure consistency and reduce code duplication.

## Building

### Prerequisites

- Rust (latest stable)
- Python 3.10+ (for Python bindings)
- `maturin` (for building Python wheels)
- `wasm-pack` (for building WASM)
- Node.js/npm (for TypeScript)

### Build Instructions

1.  **Build all Rust crates:**
    ```bash
    cargo build
    ```

2.  **Build Python bindings:**
    ```bash
    uv sync
    # or to build a wheel
    uv run maturin build --release
    ```

3.  **Build WASM/TypeScript:**
    ```bash
    cd firm_wasm
    npm install
    npm run build
    ```

## Usage

### Rust

Add `firm_rust` to your `Cargo.toml`.

```rust
use firm_rust::FirmClient;
use std::{thread, time::Duration};

fn main() {
    let mut client = FirmClient::new("/dev/ttyUSB0", 115200);
    client.start().expect("Failed to start client");

    loop {
        for packet in client.get_packets() {
            println!("{:#?}", packet);
        }
        thread::sleep(Duration::from_millis(10));
    }
}
```

### Python

```python
from firm_client import FirmClient
import time

# Using context manager (automatically starts and stops)
with FirmClient("/dev/ttyUSB0", 115200) as client:
    while True:
        packets = client.get_packets()
        for packet in packets:
            print(packet.timestamp_seconds, packet.accel_x_meters_per_s2)
        time.sleep(0.01)
```

### Web (TypeScript)

TODO!

## License

Licensed under the MIT License. See `LICENSE` file for details.
