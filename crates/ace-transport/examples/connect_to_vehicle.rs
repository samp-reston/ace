// Example: connect to a real vehicle gateway, enter extended session,
// read the VIN DID, and print the result.
//
// Run with:
//   cargo run --example connect_to_vehicle -- 192.168.1.10

use ace_client::event::ClientEvent;
use ace_doip::header::ProtocolVersion;
use ace_gateway::tester::{DoipConnectionConfig, DoipTesterEvent};
use ace_transport::doip_vehicle_driver::{DoipVehicleDriver, VehicleDriverConfig};
use std::time::Duration;

fn main() -> std::io::Result<()> {
    let gateway_ip = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "192.168.1.10".to_string());

    // region: Optional discovery
    //
    // println!("Discovering gateways...");
    // let discovery_config = DiscoveryConfig {
    //     protocol_version: DiscoveryProtocolVersion::Iso13400_2012,
    //     ..DiscoveryConfig::default()
    // };
    // let gateways = discover_gateways(&discovery_config)?;
    // for gw in &gateways {
    //     println!("  Found gateway: {}", gw);
    // }
    // endregion

    let config = VehicleDriverConfig::new(
        0x0E00,
        Duration::from_millis(1),
        65535,
        "255.255.255.255",
        ProtocolVersion::Iso13400_2012,
    );
    let mut driver = DoipVehicleDriver::new(config);

    // Connect to gateway at 0x0E80 logical address
    let gateway_addr: std::net::SocketAddr = format!("{}:13400", gateway_ip)
        .parse()
        .expect("invalid address");

    println!("Connecting to {}...", gateway_addr);

    let conn = driver.connect(gateway_addr, DoipConnectionConfig::new(0x0E00))?;

    let ecu_address: u16 = 0x0001;
    let mut vin_sent = false;

    // Main loop - drive the tester at 1ms intervals
    loop {
        driver.tick();
        let events: Vec<_> = driver.drain_events().collect();

        for (_, _, event) in events {
            match event {
                DoipTesterEvent::ActivationSucceeded => {
                    println!("Routing activation succeeded");

                    // Enter extended session
                    driver
                        .request(conn, ecu_address, &[0x10, 0x03])
                        .expect("request failed");
                }

                DoipTesterEvent::ActivationDenied { code } => {
                    eprintln!("Activation denied - code 0x{:02X}", code);
                    return Ok(());
                }

                DoipTesterEvent::ConnectionReset => {
                    eprintln!("Connection reset by vehicle");
                    return Ok(());
                }

                DoipTesterEvent::Uds(ClientEvent::PositiveResponse {
                    sid: 0x10,
                    ref data,
                }) => {
                    println!("Extended session active (P2={}ms)", {
                        let p2_ms = u16::from_be_bytes([
                            data.get(1).copied().unwrap_or(0),
                            data.get(2).copied().unwrap_or(0),
                        ]);
                        p2_ms
                    });

                    if !vin_sent {
                        // Read VIN DID 0xF190
                        driver
                            .request(conn, ecu_address, &[0x22, 0xF1, 0x90])
                            .expect("RDBI request failed");
                        vin_sent = true;
                    }
                }

                DoipTesterEvent::Uds(ClientEvent::PositiveResponse {
                    sid: 0x22,
                    ref data,
                }) => {
                    // Response: [DID_high, DID_low, data...]
                    let vin_bytes = data.get(2..).unwrap_or(&[]);
                    match core::str::from_utf8(vin_bytes) {
                        Ok(vin) => println!("VIN: {}", vin),
                        Err(_) => println!("VIN (raw): {:02X?}", vin_bytes),
                    }
                    return Ok(()); // Done
                }

                DoipTesterEvent::Uds(ClientEvent::NegativeResponse { sid, nrc }) => {
                    eprintln!("NRC 0x{:02X} for SID 0x{:02X}", nrc, sid);
                    return Ok(());
                }

                DoipTesterEvent::Uds(ClientEvent::Timeout { sid }) => {
                    eprintln!("Timeout waiting for response to SID 0x{:02X}", sid);
                    return Ok(());
                }

                _ => {}
            }
        }

        std::thread::sleep(Duration::from_millis(1));
    }
}
