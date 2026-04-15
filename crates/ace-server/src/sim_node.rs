// region: Imports

use crate::{
    handler::ServerHandler,
    security_provider::SecurityProvider,
    server::{ServerError, UdsServer},
    MAX_FRAME, MAX_OUTBOX,
};
use ace_sim::node::SimNode;

// endregion: Imports

// region: SimNode for UdsServer

impl<H, S> SimNode<MAX_FRAME, MAX_OUTBOX> for UdsServer<H, S>
where
    H: ServerHandler,
    S: SecurityProvider,
{
    type Error = ServerError<H::Error>;

    fn address(&self) -> &ace_sim::io::NodeAddress {
        UdsServer::address(self)
    }

    /// Delivers a raw UDS frame to the server.
    ///
    /// The SimRunner calls this after the SimBus delivers a message. Errors are returned to
    /// the runner - in simulation these are observed and recorded. In production the transport
    /// layer decides whether to reset or continue.
    fn handle(
        &mut self,
        src: &ace_sim::io::NodeAddress,
        data: &[u8],
        now: ace_sim::clock::Instant,
    ) -> Result<(), Self::Error> {
        UdsServer::handle(self, src, data, now)
    }

    /// Advances internal timers.
    ///
    /// Called by the SimRunner on every tick regardless of whether any messages were
    /// delivered. Drives the S3 watchdog and periodic DID scheduling.
    fn tick(&mut self, now: ace_sim::clock::Instant) -> Result<(), Self::Error> {
        UdsServer::tick(self, now)
    }

    /// Drains pending outbound frames into `out`.
    ///
    /// The SimRunner collects these after every handle and tick call and routes them back onto
    /// the SimBus.
    fn drain_outbox(
        &mut self,
        out: &mut heapless::Vec<
            (ace_sim::io::NodeAddress, heapless::Vec<u8, MAX_FRAME>),
            MAX_OUTBOX,
        >,
    ) -> usize {
        UdsServer::drain_outbox(self, out)
    }
}

// endregion: SimNode for UdsServer
