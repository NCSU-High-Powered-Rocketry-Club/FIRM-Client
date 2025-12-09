use firm_rust::FirmClient;
use std::{process::exit, thread, time::Duration};

fn main() {
    let ports = serialport::available_ports().expect("No ports found!");

    if ports.is_empty() {
        eprintln!("No serial ports detected");
        exit(1);
    }

    let port_name = &ports[0].port_name;
    println!("Connecting to {}", port_name);

    let mut client = FirmClient::new(port_name, 115_200);
    
    if let Err(e) = client.start() {
        eprintln!("Failed to start client: {}", e);
        exit(1);
    }

    loop {
        for packet in client.get_packets() {
            println!("{:#?}", packet);
        }
        
        if let Some(err) = client.check_error() {
            eprintln!("Error: {}", err);
            break;
        }
        
        thread::sleep(Duration::from_millis(10));
    }
}
