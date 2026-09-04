// region: Imports

use crate::{
    handler::ServerHandler,
    security_provider::SecurityProvider,
    server::{ServerError, UdsServer},
};
use ace_core::Vec;
use ace_sim::node::SimNode;

// endregion: Imports

// region: SimNode for UdsServer

impl<
    const MAX_FRAME: usize,
    const MAX_OUTBOX: usize,
    const MAX_SESSIONS: usize,
    const MAX_SERVICES: usize,
    const MAX_DIDS: usize,
    const MAX_SECURITY_LEVELS: usize,
    const DEFAULT_S3: u64,
    const DEFAULT_P2: u64,
    const DEFAULT_P2_EXT: u64,
    const DEFAULT_LOCKOUT: u64,
    const DEFAULT_MAX_SECURITY_ATTEMPTS: u8,
    const MAX_SEED: usize,
    const MAX_PERIODIC: usize,
    H,
    S,
> SimNode<MAX_FRAME, MAX_OUTBOX>
    for UdsServer<
        MAX_FRAME,
        MAX_OUTBOX,
        MAX_SESSIONS,
        MAX_SERVICES,
        MAX_DIDS,
        MAX_SECURITY_LEVELS,
        DEFAULT_S3,
        DEFAULT_P2,
        DEFAULT_P2_EXT,
        DEFAULT_LOCKOUT,
        DEFAULT_MAX_SECURITY_ATTEMPTS,
        MAX_SEED,
        MAX_PERIODIC,
        H,
        S,
    >
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
        out: &mut Vec<(ace_sim::io::NodeAddress, Vec<u8, MAX_FRAME>), MAX_OUTBOX>,
    ) -> usize {
        UdsServer::drain_outbox(self, out)
    }
}

// endregion: SimNode for UdsServer
