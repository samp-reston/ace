// region: Imports

// endregion: Imports

use ace_client::{client::UdsClient, config::ClientConfig};
use ace_sim::{clock::Duration, io::NodeAddress};

/// Builds a default test client with timings that comfortably exceed the default server P2 timing.
pub fn default_client(address: NodeAddress, target: NodeAddress) -> UdsClient<1> {
    let config = ClientConfig::new(address.0 as u16, target.0 as u16)
        .with_p2_timeout(Duration::from_millis(200))
        .with_p2_extended_timeout(Duration::from_millis(6_000));

    UdsClient::new(config, address)
}
