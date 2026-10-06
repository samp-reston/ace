use ace_client::client::UdsClient;
use ace_client::config::ClientConfig;
use ace_core::Vec;
use ace_core::clock::Instant;
use ace_core::io::NodeAddress;

fn main() {
    let config = ClientConfig::new(0x0E00, 0x0001);
    let mut client: UdsClient<1, 32, 4, 4, 2, 64> = UdsClient::new(config);

    client
        .request(&[0x10, 0x03], Instant::ZERO)
        .expect("request accepted");

    let mut outbox: Vec<(NodeAddress, Vec<u8, 32>), 4> = Vec::new();
    let sent = client.drain_outbox(&mut outbox).expect("outbox capacity");

    assert_eq!(sent, 1);
    assert_eq!(outbox[0].0, NodeAddress(0x0001));
    assert_eq!(outbox[0].1.as_slice(), &[0x10, 0x03]);
}
