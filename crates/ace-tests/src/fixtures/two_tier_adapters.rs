//! Adapters letting the concrete DoIP types satisfy `ace_sim::two_tier_runner`'s traits.
//!
//! None of `EcuNode`, `DoipTester`, or `DoipGateway` implement `DiagnosticNode` - the
//! fixture always called their methods directly - so none fit `SimNodeErased`/`Bridge`
//! without a wrapper. Written here rather than as blanket impls in `ace-gateway` because
//! each wraps one concrete, non-generic-over-`DiagnosticNode` type: the same reasoning
//! that keeps `SimNodeErased`'s blanket impl in the library only because it is generic
//! over `DiagnosticNode`, and keeps a hand-written impl for a specific type in the
//! consumer that uses it.
//!
//! Errors are swallowed, matching `SimNodeErased`'s own documented convention for the
//! same reason: a simulation tick that cannot report failure back through this interface
//! can only drop it, and this is not a path where a dropped error should matter - tests
//! assert on delivered state, not on tick plumbing.

// region: Imports

use ace_core::Vec;
use ace_core::clock::Instant;
use ace_core::io::{InboundFrame, LinkEvent, NodeAddress};
use ace_doip::session::ActivationAuthProvider;
use ace_gateway::ecu_node::EcuNode;
use ace_gateway::gateway::DoipGateway;
use ace_gateway::tester::DoipTester;
use ace_server::handler::ServerHandler;
use ace_server::security_provider::SecurityProvider;
use ace_sim::can_bus::CanEvent;
use ace_sim::node::SimNodeErased;
use ace_sim::two_tier_runner::{Bridge, OuterEventSink};

// endregion: Imports

// region: EcuNodeAdapter

/// Makes an ECU (plus the CAN address it answers on) a `SimNodeErased` inner-tier node.
///
/// `EcuNode` itself has no `address()` - the fixture's `EcuEntry` carries
/// `response_can_id` alongside the node instead - so the adapter carries that address
/// rather than inventing one on the node.
pub struct EcuNodeAdapter<
    'a,
    const UDS_MAX_FRAME: usize,
    const UDS_MAX_OUTBOX: usize,
    const CAN_MAX_FRAME: usize,
    const CAN_MAX_OUTBOX: usize,
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
> where
    H: ServerHandler,
    S: SecurityProvider,
{
    pub address: NodeAddress,
    pub node: &'a mut EcuNode<
        UDS_MAX_FRAME,
        UDS_MAX_OUTBOX,
        CAN_MAX_FRAME,
        CAN_MAX_OUTBOX,
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
    >,
}

impl<
    'a,
    const UDS_MAX_FRAME: usize,
    const UDS_MAX_OUTBOX: usize,
    const CAN_MAX_FRAME: usize,
    const CAN_MAX_OUTBOX: usize,
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
> SimNodeErased<CAN_MAX_FRAME, CAN_MAX_OUTBOX>
    for EcuNodeAdapter<
        'a,
        UDS_MAX_FRAME,
        UDS_MAX_OUTBOX,
        CAN_MAX_FRAME,
        CAN_MAX_OUTBOX,
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
    fn address(&self) -> &NodeAddress {
        &self.address
    }

    fn handle(&mut self, frame: &InboundFrame<'_>, now: Instant) {
        let _ = self.node.handle_can_frame(frame, now);
    }

    fn tick(&mut self, now: Instant) {
        let _ = self.node.tick(now);
    }

    fn drain_outbox(
        &mut self,
        out: &mut Vec<(NodeAddress, Vec<u8, CAN_MAX_FRAME>), CAN_MAX_OUTBOX>,
    ) -> usize {
        self.node.drain_can_outbox(out).unwrap_or(0)
    }
}

// endregion: EcuNodeAdapter

// region: DoipTesterAdapter

/// Makes `DoipTester` a `SimNodeErased` + `OuterEventSink` outer-tier node.
pub struct DoipTesterAdapter<
    'a,
    const MAX_GATEWAYS: usize,
    const MAX_TARGETS: usize,
    const TCP_MAX_FRAME: usize,
    const TCP_MAX_OUTBOX: usize,
    const MAX_CONNECTION_ACTIONS: usize,
    const PENDING: usize,
    const SIM_MAX_FRAME: usize,
    const SIM_MAX_OUTBOX: usize,
    const MAX_TARGET_EVENTS: usize,
    const PERIODIC_DIDS: usize,
    const MAX_DATA: usize,
>(
    pub  &'a mut DoipTester<
        MAX_GATEWAYS,
        MAX_TARGETS,
        TCP_MAX_FRAME,
        TCP_MAX_OUTBOX,
        MAX_CONNECTION_ACTIONS,
        PENDING,
        SIM_MAX_FRAME,
        SIM_MAX_OUTBOX,
        MAX_TARGET_EVENTS,
        PERIODIC_DIDS,
        MAX_DATA,
    >,
);

impl<
    'a,
    const MAX_GATEWAYS: usize,
    const MAX_TARGETS: usize,
    const TCP_MAX_FRAME: usize,
    const TCP_MAX_OUTBOX: usize,
    const MAX_CONNECTION_ACTIONS: usize,
    const PENDING: usize,
    const SIM_MAX_FRAME: usize,
    const SIM_MAX_OUTBOX: usize,
    const MAX_TARGET_EVENTS: usize,
    const PERIODIC_DIDS: usize,
    const MAX_DATA: usize,
> SimNodeErased<TCP_MAX_FRAME, TCP_MAX_OUTBOX>
    for DoipTesterAdapter<
        'a,
        MAX_GATEWAYS,
        MAX_TARGETS,
        TCP_MAX_FRAME,
        TCP_MAX_OUTBOX,
        MAX_CONNECTION_ACTIONS,
        PENDING,
        SIM_MAX_FRAME,
        SIM_MAX_OUTBOX,
        MAX_TARGET_EVENTS,
        PERIODIC_DIDS,
        MAX_DATA,
    >
{
    fn address(&self) -> &NodeAddress {
        self.0.address()
    }

    fn handle(&mut self, frame: &InboundFrame<'_>, now: Instant) {
        let _ = self.0.handle(frame, now);
    }

    fn tick(&mut self, now: Instant) {
        let _ = self.0.tick(now);
    }

    fn drain_outbox(
        &mut self,
        out: &mut Vec<(NodeAddress, Vec<u8, TCP_MAX_FRAME>), TCP_MAX_OUTBOX>,
    ) -> usize {
        self.0.drain_outbox(out).unwrap_or(0)
    }
}

impl<
    'a,
    const MAX_GATEWAYS: usize,
    const MAX_TARGETS: usize,
    const TCP_MAX_FRAME: usize,
    const TCP_MAX_OUTBOX: usize,
    const MAX_CONNECTION_ACTIONS: usize,
    const PENDING: usize,
    const SIM_MAX_FRAME: usize,
    const SIM_MAX_OUTBOX: usize,
    const MAX_TARGET_EVENTS: usize,
    const PERIODIC_DIDS: usize,
    const MAX_DATA: usize,
> OuterEventSink
    for DoipTesterAdapter<
        'a,
        MAX_GATEWAYS,
        MAX_TARGETS,
        TCP_MAX_FRAME,
        TCP_MAX_OUTBOX,
        MAX_CONNECTION_ACTIONS,
        PENDING,
        SIM_MAX_FRAME,
        SIM_MAX_OUTBOX,
        MAX_TARGET_EVENTS,
        PERIODIC_DIDS,
        MAX_DATA,
    >
{
    fn on_outer_event(&mut self, event: &LinkEvent, now: Instant) {
        let _ = self.0.on_tcp_event(event, now);
    }
}

// endregion: DoipTesterAdapter

// region: DoipGatewayAdapter

/// Makes `DoipGateway` a `Bridge` between the TCP (outer) and CAN (inner) tiers.
#[allow(clippy::type_complexity)]
pub struct DoipGatewayAdapter<
    'a,
    const CAN_MAX_FRAME: usize,
    const ISOTP_MAX_NODES: usize,
    const TCP_MAX_FRAME: usize,
    const TCP_MAX_OUTBOX: usize,
    const CAN_MAX_OUTBOX: usize,
    const MAX_TESTERS: usize,
    const MAX_FRAME: usize,
    const UDS_MAX_FRAME: usize,
    const MAX_CONNECTION_ACTIONS: usize,
    const MAX_ACTIVATION_TYPES: usize,
    const MAX_NODES: usize,
    A,
>(
    pub  &'a mut DoipGateway<
        CAN_MAX_FRAME,
        ISOTP_MAX_NODES,
        TCP_MAX_FRAME,
        TCP_MAX_OUTBOX,
        CAN_MAX_OUTBOX,
        MAX_TESTERS,
        MAX_FRAME,
        UDS_MAX_FRAME,
        MAX_CONNECTION_ACTIONS,
        MAX_ACTIVATION_TYPES,
        MAX_NODES,
        A,
    >,
)
where
    A: ActivationAuthProvider + Clone;

impl<
    'a,
    const CAN_MAX_FRAME: usize,
    const ISOTP_MAX_NODES: usize,
    const TCP_MAX_FRAME: usize,
    const TCP_MAX_OUTBOX: usize,
    const CAN_MAX_OUTBOX: usize,
    const MAX_TESTERS: usize,
    const MAX_FRAME: usize,
    const UDS_MAX_FRAME: usize,
    const MAX_CONNECTION_ACTIONS: usize,
    const MAX_ACTIVATION_TYPES: usize,
    const MAX_NODES: usize,
    A,
> Bridge<TCP_MAX_FRAME, TCP_MAX_OUTBOX, CAN_MAX_FRAME, CAN_MAX_OUTBOX>
    for DoipGatewayAdapter<
        'a,
        CAN_MAX_FRAME,
        ISOTP_MAX_NODES,
        TCP_MAX_FRAME,
        TCP_MAX_OUTBOX,
        CAN_MAX_OUTBOX,
        MAX_TESTERS,
        MAX_FRAME,
        UDS_MAX_FRAME,
        MAX_CONNECTION_ACTIONS,
        MAX_ACTIVATION_TYPES,
        MAX_NODES,
        A,
    >
where
    A: ActivationAuthProvider + Clone,
{
    fn outer_address(&self) -> &NodeAddress {
        self.0.address()
    }

    fn handle_outer(&mut self, frame: &InboundFrame<'_>, now: Instant) {
        let _ = self.0.handle_tcp(frame, now);
    }

    fn handle_inner(&mut self, frame: &InboundFrame<'_>, now: Instant) {
        let _ = self.0.handle_can_frame(frame, now);
    }

    fn tick(&mut self, now: Instant) {
        let _ = self.0.tick(now);
    }

    fn drain_outer_outbox(
        &mut self,
        out: &mut Vec<(NodeAddress, Vec<u8, TCP_MAX_FRAME>), TCP_MAX_OUTBOX>,
    ) -> usize {
        self.0.drain_tcp_outbox(out).unwrap_or(0)
    }

    fn drain_inner_outbox(
        &mut self,
        out: &mut Vec<(NodeAddress, Vec<u8, CAN_MAX_FRAME>), CAN_MAX_OUTBOX>,
    ) -> usize {
        self.0.drain_can_outbox(out).unwrap_or(0)
    }

    // The gateway consumes neither TCP link events nor CAN bus events today - see
    // `ace-gateway/src/gateway.rs`, which forwards no such thing. No-ops here rather
    // than silently discarding something meaningful.
    fn on_outer_event(&mut self, _event: &LinkEvent, _now: Instant) {}

    fn on_inner_event(&mut self, _event: &CanEvent, _now: Instant) {}
}

// endregion: DoipGatewayAdapter
