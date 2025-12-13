use firm_rust::FIRMClient;
use std::{process::exit, thread, time::Duration};

fn main() {
    let ports = serialport::available_ports().expect("No ports found!");

    if ports.is_empty() {
        eprintln!("No serial ports detected");
        exit(1);
    }

    let port_name = &ports[0].port_name;
    println!("Connecting to {}", port_name);

    let mut client = match FIRMClient::new(port_name, 115_200, 0.1) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to create client: {}", e);
            exit(1);
        }
    };

    client.start();

    loop {
        while let Ok(packet) = client.get_data_packets(Some(Duration::from_millis(100))) {
            println!("{:#?}", packet);
        }
        
        if let Some(err) = client.check_error() {
            eprintln!("Error: {}", err);
            break;
        }
        
        thread::sleep(Duration::from_millis(10));
    }
}
