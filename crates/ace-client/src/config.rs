// region: Imports

use ace_sim::clock::Duration;

// endregion: Imports

// region: Client Config

/// Configuration for a UDS tester client.
///
/// Controls timing behaviour only - the client tracks no session or security state. All session
/// and security management is the responsibility of the caller.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// P2 client - time to wait for a response before declaring timeout. Should match or slightly
    /// exceed the server's P2 server timing.
    pub p2_timeout: Duration,

    /// P2* client - extended timeout after receiving a 0x78 Response Pending. Should match or
    /// slightly exceed the server's P2* server timing.
    pub p2_extended_timeout: Duration,

    /// Physical address of this client node.
    pub physical_address: u16,

    /// Physical address of the target server.
    pub target_address: u16,
}

impl ClientConfig {
    pub fn new(physical_address: u16, target_address: u16) -> Self {
        Self {
            p2_timeout: Duration::from_millis(150),
            p2_extended_timeout: Duration::from_millis(5_000),
            physical_address,
            target_address,
        }
    }

    pub fn with_p2_timeout(mut self, timeout: Duration) -> Self {
        self.p2_timeout = timeout;
        self
    }

    pub fn with_p2_extended_timeout(mut self, timeout: Duration) -> Self {
        self.p2_extended_timeout = timeout;
        self
    }
}

// endregion: Client Config
