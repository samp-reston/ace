// region: Imports

use crate::{client::UdsClient, ClientError};
use ace_sim::node::SimNode;

// endregion: Imports

// region: Capacity Constants

/// Maximum UDS frame payload bytes = matches ace-server for bus compatibility.
pub const SIM_MAX_FRAME: usize = 4096;

/// Maximum outbox depth.
pub const SIM_MAX_OUTBOX: usize = 16;

// endregion: Capacity Constants

// region: SimNode for UdsClient

impl<const N: usize> SimNode<SIM_MAX_FRAME, SIM_MAX_OUTBOX> for UdsClient<N> {
    type Error = ClientError;

    fn address(&self) -> &ace_sim::io::NodeAddress {
        UdsClient::address(self)
    }

    fn handle(
        &mut self,
        src: &ace_sim::io::NodeAddress,
        data: &[u8],
        now: ace_sim::clock::Instant,
    ) -> Result<(), Self::Error> {
        UdsClient::handle(self, src, data, now)
    }

    fn tick(&mut self, now: ace_sim::clock::Instant) -> Result<(), Self::Error> {
        UdsClient::tick(self, now)
    }

    fn drain_outbox(
        &mut self,
        out: &mut heapless::Vec<
            (ace_sim::io::NodeAddress, heapless::Vec<u8, SIM_MAX_FRAME>),
            SIM_MAX_OUTBOX,
        >,
    ) -> usize {
        UdsClient::drain_outbox(self, out)
    }
}

// endregion: SimNode for UdsClient
