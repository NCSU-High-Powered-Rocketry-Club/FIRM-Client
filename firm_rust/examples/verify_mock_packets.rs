use anyhow::Result;
use firm_core::client_packets::FIRMMockPacket;
use firm_core::constants::mock_constants::FIRMMockPacketType;
use firm_core::constants::mock_constants::HEADER_TOTAL_SIZE;
use firm_core::framed_packet::Framed;
use firm_core::mock::MockParser;
use std::fs::File;
use std::io::Read;

// Edit these before running.
const LOG_PATH: &str = r"C:\Users\jackg\Downloads\LOG47.TXT";
const CHUNK_SIZE: usize = 65000;

fn main() -> Result<()> {
    let mut parser = MockParser::new();

    let mut file = File::open(LOG_PATH)?;
    
    let mut header = vec![0u8; HEADER_TOTAL_SIZE];
    file.read_exact(&mut header)?;

    if header.starts_with(b"FIRM") {
        println!("✅ File header looks correct: FIRM...");
    } else {
        println!("❌ WRONG FILE! Header starts with: {:02X?}", &header[0..4]);
        return Ok(()); // Stop here
    }
    
    parser.read_header(&header);

    let mut buf = vec![0u8; CHUNK_SIZE];

    let mut count_total = 0usize;
    let mut count_b = 0usize;
    let mut count_i = 0usize;
    let mut count_m = 0usize;

    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }

        parser.parse_bytes(&buf[..n]);

        // Just verifies the round-trip serialization/parsing of packets
        while let Some((pkt, delay_s)) = parser.get_packet_with_delay() {
            let bytes = pkt.to_bytes();
            let parsed = FIRMMockPacket::from_bytes(&bytes)
                .expect("failed to parse bytes we just serialized (header/len/crc mismatch)");
            assert_eq!(parsed.payload(), pkt.payload());

            count_total += 1;
            match parsed.packet_type() {
                FIRMMockPacketType::BarometerPacket => count_b += 1,
                FIRMMockPacketType::IMUPacket => count_i += 1,
                FIRMMockPacketType::MagnetometerPacket => count_m += 1,
                other => println!("Unexpected packet type: {other:?}"),
            }

            if count_total <= 500 {
                let id_char = match parsed.packet_type() {
                    FIRMMockPacketType::HeaderPacket => 'H',
                    FIRMMockPacketType::BarometerPacket => 'B',
                    FIRMMockPacketType::IMUPacket => 'I',
                    FIRMMockPacketType::MagnetometerPacket => 'M',
                };
                println!(
                    "#{count_total} id={} payload_len={} delay_s={:.8}",
                    id_char,
                    parsed.payload().len(),
                    delay_s
                );
            }
        }

        // TODO only testing 1 loop for now
        break;
    }

    // Drain any remaining packets buffered by the parser.
    while let Some((pkt, _delay_s)) = parser.get_packet_with_delay() {
        let bytes = pkt.to_bytes();
        let parsed = FIRMMockPacket::from_bytes(&bytes)
            .expect("failed to parse bytes we just serialized (header/len/crc mismatch)");
        assert_eq!(parsed.payload(), pkt.payload());

        count_total += 1;
        match parsed.packet_type() {
            FIRMMockPacketType::BarometerPacket => count_b += 1,
            FIRMMockPacketType::IMUPacket => count_i += 1,
            FIRMMockPacketType::MagnetometerPacket => count_m += 1,
            _ => {}
        }
    }

    println!(
        "OK: total={count_total} B={count_b} I={count_i} M={count_m} (round-trip header/len/crc verified)"
    );

    Ok(())
}
