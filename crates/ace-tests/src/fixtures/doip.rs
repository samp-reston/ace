//! DoIP-specific fixtures for DST scenarios.
//!
//! Provides concrete implementations of:
//!     - ActivationAuthProvider - for the DoipGateway
//!     - DoipDstScenario - full stack wiring
//!     - Helper builders

// region: Imports
use ace_can::IsoTpAddressingMode;
use ace_client::{SIM_MAX_FRAME, SIM_MAX_OUTBOX};
use ace_doip::{
    payload::ActivationType,
    session::{ActivationAuthProvider, ActivationDenialReason},
};
use ace_gateway::{
    config::{CanNodeEntry, GatewayConfig},
    gateway::{DoipGateway, CAN_MAX_FRAME, CAN_MAX_OUTBOX, TCP_MAX_FRAME, TCP_MAX_OUTBOX},
    isotp_node::{IsoTpNode, ISOTP_MAX_FRAME, ISOTP_MAX_OUT, ISOTP_MAX_UDS},
    tester::{
        ConnectionId, DoipConnectionConfig, DoipConnectionPhase, DoipTester, DoipTesterError,
        DoipTesterEvent, TargetId,
    },
};
use ace_server::server::UdsServer;
use ace_sim::{
    can_bus::{CanFaultConfig, CanSimBus},
    clock::Duration,
    io::NodeAddress,
    tcp_bus::{TcpFaultConfig, TcpSimBus},
};

use crate::fixtures::{server::default_server, TestHandler, TestSecurityProvider};

// endregion: Imports

// region: TestActivationAuthProvider

/// Always-allow activation auth provider for DST tests.
///
/// Kept separate from the UDS-layer security provider to maintain clear separation of
/// responsibilities:
///     - `TestActivationAuthProvider` - DoIP routing activation gate
///     - `TestSecurityProvider` - UDS SecurityAccess seed/key
#[derive(Clone)]
pub struct TestActivationAuthProvider;

impl ActivationAuthProvider for TestActivationAuthProvider {
    fn authenticate(
        &mut self,
        _source_address: u16,
        _oem_data: &[u8],
    ) -> Result<(), ace_doip::session::ActivationDenialReason> {
        Ok(())
    }
}

/// Denial activation auth provider - denies all requests. Used to test activation failure paths.
#[derive(Clone)]
pub struct DenyActivationAuthProvider {
    pub reason: ActivationDenialReason,
}

impl ActivationAuthProvider for DenyActivationAuthProvider {
    fn authenticate(
        &mut self,
        _source_address: u16,
        _oem_data: &[u8],
    ) -> Result<(), ActivationDenialReason> {
        Err(self.reason.clone())
    }
}

// endregion: TestActivationAuthProvider

// region: DoipScenarioConfig

/// Configuration for the DoIP DST scenario tick behaviour.
#[derive(Debug, Clone)]
pub struct DoipScenarioConfig {
    /// Duration of each TCP bus tick.
    pub tcp_tick: Duration,

    /// Duration of each CAN bus tick.
    pub can_tick: Duration,

    /// Number of CAN ticks per TCP tick.
    pub can_ticks_per_tcp: usize,
}

impl Default for DoipScenarioConfig {
    fn default() -> Self {
        Self {
            tcp_tick: Duration::from_millis(1),
            can_tick: Duration::from_micros(100),
            can_ticks_per_tcp: 10,
        }
    }
}

// endregion: DoipScenarioConfig

// region: EcuEntry

/// A single ECU node wired into the scenario.
pub struct EcuEntry {
    /// ECU logical address (DoIP)
    pub logical_address: u16,

    /// CAN request ID (gateway -> ECU)
    pub request_can_id: u32,

    /// CAN response ID (ECU -> gateway)
    pub response_can_id: u32,
    pub isotp: IsoTpNode,
    pub server: UdsServer<TestHandler, TestSecurityProvider>,
}

// endregion: EcuEntry

// region: GatewayEntry

/// A single gateway wired into the scenario.
pub struct GatewayEntry {
    /// Gateway logical address (DoIP).
    pub logical_address: u16,
    pub gateway_addr: NodeAddress,
    pub gateway: DoipGateway<TestActivationAuthProvider, 1>,
    pub ecus: heapless::Vec<EcuEntry, 8>,

    /// `ConnectionId` the tester uses to reach this gateway.
    pub conn_id: ConnectionId,
}

// endregion: GatewayEntry

// region: DoipDstScenario

/// Full DoIP -> CAN diagnostic simulation scenario.
///
/// Stack: DoipTester - TcpSimBus - DoipGateway - CanSimBus - IsoTpNode - UdsServer
///
/// The scenario drives both buses explicitly - the gateway's TCP and CAN faces are called directly
/// with the appropriate messages. This keeps the routing transparent and deterministic.
pub struct DoipDstScenario {
    pub config: DoipScenarioConfig,
    pub tcp_bus: TcpSimBus<TCP_MAX_FRAME, TCP_MAX_OUTBOX>,
    pub can_bus: CanSimBus<CAN_MAX_FRAME, CAN_MAX_OUTBOX>,
    pub tester: DoipTester<4, 8>,
    pub gateways: heapless::Vec<GatewayEntry, 4>,
}

impl DoipDstScenario {
    pub fn baseline(seed: u64) -> Self {
        DoipDstScenarioBuilder::new(seed).build()
    }

    pub fn light(seed: u64) -> Self {
        DoipDstScenarioBuilder::new(seed)
            .with_tcp_faults(TcpFaultConfig::light())
            .with_can_faults(CanFaultConfig::light())
            .build()
    }

    pub fn chaos(seed: u64) -> Self {
        DoipDstScenarioBuilder::new(seed)
            .with_tcp_faults(TcpFaultConfig::chaos())
            .with_can_faults(CanFaultConfig::chaos())
            .build()
    }

    /// Returns the `ConnectionId` for the first gateway, panics if none.
    pub fn conn_id(&mut self) -> ConnectionId {
        self.gateways
            .first()
            .expect("scenario has no gateways")
            .conn_id
    }

    /// Returns the logical address of the frst ECU on the first gateway.
    pub fn first_ecu(&self) -> u16 {
        self.gateways
            .first()
            .expect("scenario has no gateways")
            .ecus
            .first()
            .expect("first gateway has no ECUs")
            .logical_address
    }

    // region: Tick

    /// Initiates the TCP connection from the tester to the gateway.
    ///
    /// Must be called before the first tick. The tester will send its `RoutingActivationRequest`
    /// automatically on `ConnectionEstablished`.
    pub fn connect(&mut self) {
        let tester_addr = NodeAddress(0x0e00);

        for gw in &self.gateways {
            self.tcp_bus
                .connect(tester_addr.clone(), gw.gateway_addr.clone());
        }
    }

    /// Advances the simulation by on full step.
    ///
    /// One step = one TCP tick + N CAN ticks.
    /// Order:
    ///     1. TCP Bus tick - deliver DoIP frames.
    ///     2. Route TCP messages to tester or gateway TCP face
    ///     3. Collect TCP events, notify tester
    ///     4. Collect gateway TCP outbox -> TCP bus
    ///     5. Collect gateway CAN outbox -> CAN bus
    ///     6. N * CAN bus ticks:
    ///         a. Deliver CAN frames to ISO-TP node
    ///         b. Collect ISO-TP CAN outbox -> CAN bus
    ///         c. Route ISO-TP UDS outbox -> UdsServer
    ///         d. Collect UdsServer outbox -> ISO-TP node
    ///         e. Route ISO-TP assembled UDS -> gateway CAN face
    ///         f. Collect gateway TCP outbox -> TCP bus (response path)
    pub fn tick(&mut self) {
        let tester_addr = NodeAddress(0x0E00);
        let now = self.tcp_bus.now();

        let tcp_delivered = self.tcp_bus.tick(self.config.tcp_tick);
        let tcp_events: heapless::Vec<_, 16> = self.tcp_bus.drain_events().collect();
        for event in &tcp_events {
            self.tester.on_tcp_event(event, now);
        }

        for envelope in &tcp_delivered {
            let gw_hit = self
                .gateways
                .iter_mut()
                .find(|g| g.gateway_addr == envelope.dst);

            if let Some(gw) = gw_hit {
                let _ = gw.gateway.handle_tcp(&envelope.src, &envelope.data, now);
            } else if envelope.dst == tester_addr {
                let _ = self.tester.handle(&envelope.src, &envelope.data, now);
            }
        }

        self.tester.tick(now);

        let mut tester_out: heapless::Vec<
            (NodeAddress, heapless::Vec<u8, TCP_MAX_FRAME>),
            TCP_MAX_OUTBOX,
        > = heapless::Vec::new();
        self.tester.drain_outbox(&mut tester_out);
        for (dst, data) in &tester_out {
            self.tcp_bus.send(tester_addr.clone(), dst.clone(), data);
        }

        for gw in self.gateways.iter_mut() {
            let _ = gw.gateway.tick(now);

            let mut gw_tcp_out: heapless::Vec<
                (NodeAddress, heapless::Vec<u8, TCP_MAX_FRAME>),
                TCP_MAX_OUTBOX,
            > = heapless::Vec::new();
            gw.gateway.drain_tcp_outbox(&mut gw_tcp_out);
            for (dst, data) in &gw_tcp_out {
                self.tcp_bus
                    .send(gw.gateway_addr.clone(), dst.clone(), data);
            }

            let mut gw_can_out: heapless::Vec<
                (NodeAddress, heapless::Vec<u8, CAN_MAX_FRAME>),
                CAN_MAX_OUTBOX,
            > = heapless::Vec::new();
            gw.gateway.drain_can_outbox(&mut gw_can_out);
            for (dst, data) in &gw_can_out {
                if let Some(ecu) = gw
                    .ecus
                    .iter_mut()
                    .find(|e| NodeAddress(e.request_can_id) == *dst)
                {
                    let _ = ecu.isotp.handle_from_gateway(data, now);
                }
            }
        }

        for _ in 0..self.config.can_ticks_per_tcp {
            let can_now = self.can_bus.now();
            let can_delivered = self.can_bus.tick(self.config.can_tick);

            for gw in self.gateways.iter_mut() {
                for ecu in gw.ecus.iter_mut() {
                    for envelope in &can_delivered {
                        if envelope.dst == NodeAddress(ecu.request_can_id) {
                            let _ = ecu.isotp.handle_from_gateway(&envelope.data, can_now);
                        } else if envelope.dst == NodeAddress(ecu.response_can_id) {
                            let _ = ecu.isotp.handle_from_ecu(&envelope.data, can_now);
                        }
                    }

                    let mut isotp_can_out: heapless::Vec<
                        (NodeAddress, heapless::Vec<u8, ISOTP_MAX_FRAME>),
                        ISOTP_MAX_OUT,
                    > = heapless::Vec::new();
                    ecu.isotp.drain_can_outbox(&mut isotp_can_out);
                    for (dst, data) in &isotp_can_out {
                        self.can_bus
                            .send(NodeAddress(ecu.request_can_id), dst.clone(), data);
                    }

                    let mut isotp_uds_out: heapless::Vec<
                        (NodeAddress, heapless::Vec<u8, ISOTP_MAX_UDS>),
                        4,
                    > = heapless::Vec::new();
                    ecu.isotp.drain_uds_outbox(&mut isotp_uds_out);
                    for (_, data) in &isotp_uds_out {
                        let _ = ecu
                            .server
                            .handle(&NodeAddress(ecu.request_can_id), data, can_now);
                    }

                    let _ = ecu.server.tick(can_now);

                    let mut srv_out: heapless::Vec<
                        (NodeAddress, heapless::Vec<u8, SIM_MAX_FRAME>),
                        SIM_MAX_OUTBOX,
                    > = heapless::Vec::new();
                    ecu.server.drain_outbox(&mut srv_out);
                    for (_, data) in &srv_out {
                        let _ = ecu.isotp.handle_from_ecu(data, can_now);
                    }

                    let mut isotp_resp_out: heapless::Vec<
                        (NodeAddress, heapless::Vec<u8, ISOTP_MAX_UDS>),
                        4,
                    > = heapless::Vec::new();
                    ecu.isotp.drain_uds_outbox(&mut isotp_resp_out);
                    for (_, data) in &isotp_resp_out {
                        let _ =
                            gw.gateway
                                .handle_can(&NodeAddress(ecu.response_can_id), data, can_now);
                    }
                }

                let mut gw_tcp_resp: heapless::Vec<
                    (NodeAddress, heapless::Vec<u8, TCP_MAX_FRAME>),
                    TCP_MAX_OUTBOX,
                > = heapless::Vec::new();
                gw.gateway.drain_tcp_outbox(&mut gw_tcp_resp);
                for (dst, data) in &gw_tcp_resp {
                    let _ = self
                        .tcp_bus
                        .send(gw.gateway_addr.clone(), dst.clone(), data);
                }
            }
        }
    }

    pub fn tick_n(&mut self, n: usize) {
        for _ in 0..n {
            self.tick();
        }
    }

    pub fn tick_until_quiet(&mut self, max_ticks: usize) -> usize {
        for i in 0..max_ticks {
            if self.gateways.iter().all(|gw| {
                self.tester
                    .connection_phase(gw.conn_id)
                    .map(|p| *p == DoipConnectionPhase::Active)
                    .unwrap_or(false)
            }) {
                return i;
            }

            self.tick();
        }

        max_ticks
    }

    // endregion: Tick

    // region: Convenience Helpers

    /// Sends a UDS request from the tester to the ECU via DoIP.
    pub fn request(
        &mut self,
        conn_id: ConnectionId,
        target_address: u16,
        uds_data: &[u8],
    ) -> Result<(), DoipTesterError> {
        let now = self.tcp_bus.now();
        self.tester.request(conn_id, target_address, uds_data, now)
    }

    /// Sends a UDS request to the first ECU on the gateway. convenience wrapper for single-gateway
    /// single-ECU tests.
    pub fn request_default(&mut self, uds_data: &[u8]) -> Result<(), DoipTesterError> {
        let conn_id = self.conn_id();
        let target_address = self.first_ecu();
        self.request(conn_id, target_address, uds_data)
    }

    /// Returns true if the tester's connection to the gateway is active.
    pub fn is_activated(&self) -> bool {
        self.gateways
            .first()
            .map(|gw| {
                self.tester
                    .connection_phase(gw.conn_id)
                    .map(|p| *p == DoipConnectionPhase::Active)
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    }

    /// Drains all tester events
    pub fn drain_events(
        &mut self,
    ) -> heapless::Vec<(ConnectionId, TargetId, DoipTesterEvent), 128> {
        self.tester.drain_events().collect()
    }

    // endregion: Convenience Helpers
}

// endregion: DoipDstScenario

// region: EcuNodeConfig

/// Builder config for a single ECU node.
#[derive(Clone)]
pub struct EcuNodeConfig {
    pub logical_address: u16,
    pub request_can_id: u32,
    pub response_can_id: u32,
    pub functional_can_id: u32,
    pub addressing_mode: IsoTpAddressingMode,
}

impl EcuNodeConfig {
    pub fn new(
        logical_address: u16,
        request_can_id: u32,
        response_can_id: u32,
        functional_can_id: u32,
    ) -> Self {
        Self {
            logical_address,
            request_can_id,
            response_can_id,
            functional_can_id,
            addressing_mode: IsoTpAddressingMode::Normal,
        }
    }

    pub fn with_addressing(mut self, mode: IsoTpAddressingMode) -> Self {
        self.addressing_mode = mode;
        self
    }
}

// endregion: EcuNodeConfig

// region: GatewayNodeConfig

/// Builder config for a single gateway and its ECUs.
pub struct GatewayNodeConfig {
    pub logical_address: u16,
    pub tester_address: u16,
    pub activation_type: ActivationType,
    pub ecus: heapless::Vec<EcuNodeConfig, 8>,
}

impl GatewayNodeConfig {
    pub fn new(logical_address: u16, tester_address: u16) -> Self {
        Self {
            logical_address,
            tester_address,
            activation_type: ActivationType::Default,
            ecus: heapless::Vec::new(),
        }
    }

    pub fn with_ecu(mut self, ecu: EcuNodeConfig) -> Self {
        let _ = self.ecus.push(ecu);
        self
    }

    pub fn with_activation_type(mut self, t: ActivationType) -> Self {
        self.activation_type = t;
        self
    }
}

// endregion: GatewayNodeConfig

// region: DoipDstScenarioBuilder

/// Builder for `DoipDstScenario`.
///
/// Supports multiple gateways each with multiple ECUs.
///
/// # Example - single gateway, two ECUs
///
/// ```no_run
/// let s = DoipDstScenarioBuilder::new(0)
///     .with_gateway(GatewayNodeConfig::new(0x0E80, 0x0E00)
///         .with_ecu(EcuNodeConfig::new(0x0001, 0x7E0, 0x7E8, 0x7DF))
///         .with_ecu(EcuNodeConfig::new(0x0002, 0x7E2, 0x7EA, 0x7DF))
///     )
///     .build();
/// ```
///
/// # Example - two gateways
///
/// ```no_run
/// let s = DoipDstScenarioBuilder::new(0)
///     .with_gateway(GatewayNodeConfig::new(0x0E80, 0x0E00)
///         .with_ecu(EcuNodeConfig::new(0x0001, 0x7E0, 0x7E8, 0x7DF))
///     )
///     .add_gateway(GatewayNodeConfig::new(0x0E81, 0x0E00)
///         .with_ecu(EcuNodeConfig::new(0x0002, 0x7E4, 0x7EC, 0x7DF))
///     )
///     .build();
/// ```
pub struct DoipDstScenarioBuilder {
    seed: u64,
    tcp_faults: TcpFaultConfig,
    can_faults: CanFaultConfig,
    tick_config: DoipScenarioConfig,
    gateways: heapless::Vec<GatewayNodeConfig, 4>,
}

// Default ECU for the no-argument builder path
const DEFAULT_TESTER_ADDR: u16 = 0x0E00;
const DEFAULT_GATEWAY_ADDR: u16 = 0x0E80;
const DEFAULT_ECU_ADDR: u16 = 0x0001;
const DEFAULT_REQ_CAN_ID: u32 = 0x7E0;
const DEFAULT_RESP_CAN_ID: u32 = 0x7E8;
const DEFAULT_FUNC_CAN_ID: u32 = 0x7EF;

impl DoipDstScenarioBuilder {
    /// Creates a builder with the default single-gateway single-ECU topology.
    pub fn new(seed: u64) -> Self {
        let default_gw = GatewayNodeConfig::new(DEFAULT_GATEWAY_ADDR, DEFAULT_TESTER_ADDR)
            .with_ecu(EcuNodeConfig::new(
                DEFAULT_ECU_ADDR,
                DEFAULT_REQ_CAN_ID,
                DEFAULT_RESP_CAN_ID,
                DEFAULT_FUNC_CAN_ID,
            ));

        Self {
            seed,
            tcp_faults: TcpFaultConfig::none(),
            can_faults: CanFaultConfig::none(),
            tick_config: DoipScenarioConfig::default(),
            gateways: {
                let mut v = heapless::Vec::new();
                let _ = v.push(default_gw);
                v
            },
        }
    }

    /// Replaces all gateways with the given one. Use `with_gateway` to add gateways on top of the
    /// default.
    pub fn with_gateway(mut self, gw: GatewayNodeConfig) -> Self {
        self.gateways.clear();
        let _ = self.gateways.push(gw);
        self
    }

    /// Adds and additional gateway alongside existing ones.
    pub fn add_gateway(mut self, gw: GatewayNodeConfig) -> Self {
        let _ = self.gateways.push(gw);
        self
    }

    pub fn with_tcp_faults(mut self, f: TcpFaultConfig) -> Self {
        self.tcp_faults = f;
        self
    }

    pub fn with_can_faults(mut self, f: CanFaultConfig) -> Self {
        self.can_faults = f;
        self
    }

    pub fn with_tick_config(mut self, c: DoipScenarioConfig) -> Self {
        self.tick_config = c;
        self
    }

    /// Builds the `DoipDstScenario`.
    pub fn build(self) -> DoipDstScenario {
        let tcp_bus = TcpSimBus::new(self.seed, self.tcp_faults);
        let can_bus = CanSimBus::new(self.seed.wrapping_add(1), self.can_faults);

        let tester_address = self
            .gateways
            .first()
            .map(|g| g.tester_address)
            .unwrap_or(DEFAULT_TESTER_ADDR);

        let mut tester = DoipTester::new(tester_address, NodeAddress(tester_address as u32));
        let mut gateway_entries: heapless::Vec<GatewayEntry, 4> = heapless::Vec::new();

        for gw_config in &self.gateways {
            let gw_addr = NodeAddress(gw_config.logical_address as u32);

            let mut gw_builder =
                GatewayConfig::new(gw_config.logical_address).with_tester(gw_config.tester_address);

            for ecu in &gw_config.ecus {
                gw_builder = gw_builder.with_node(CanNodeEntry {
                    logical_address: ecu.logical_address,
                    request_can_id: ecu.request_can_id,
                    response_can_id: ecu.response_can_id,
                    functional_can_id: ecu.functional_can_id,
                });
            }

            let gateway = DoipGateway::new(gw_builder, TestActivationAuthProvider, gw_addr.clone());

            let conn_config = DoipConnectionConfig::new(gw_config.logical_address)
                .with_activation_type(gw_config.activation_type.clone());

            let conn_id = tester
                .open_connection(conn_config)
                .expect("connection slot available");

            let mut ecu_entries: heapless::Vec<EcuEntry, 8> = heapless::Vec::new();
            for ecu in &gw_config.ecus {
                let isotp = IsoTpNode::new(
                    ecu.request_can_id,
                    ecu.response_can_id,
                    ecu.addressing_mode.clone(),
                );
                let server = default_server(NodeAddress(ecu.logical_address as u32));
                let _ = ecu_entries.push(EcuEntry {
                    logical_address: ecu.logical_address,
                    request_can_id: ecu.request_can_id,
                    response_can_id: ecu.response_can_id,
                    isotp,
                    server,
                });
            }

            let _ = gateway_entries.push(GatewayEntry {
                logical_address: gw_config.logical_address,
                gateway_addr: gw_addr,
                gateway,
                ecus: ecu_entries,
                conn_id,
            });
        }

        DoipDstScenario {
            config: self.tick_config,
            tcp_bus,
            can_bus,
            tester,
            gateways: gateway_entries,
        }
    }
}

// endregion: DoipDstScenarioBuilder

// region: Convenience address constants for single-gateway tests

pub const TESTER_ADDR: u16 = DEFAULT_TESTER_ADDR;
pub const GATEWAY_ADDR: u16 = DEFAULT_GATEWAY_ADDR;
pub const DEFAULT_ECU: u16 = DEFAULT_ECU_ADDR;

// endregion: Convenience address constants for single-gateway tests
