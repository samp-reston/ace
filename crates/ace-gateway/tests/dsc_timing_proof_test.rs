//! Regression test for the DSC (`DiagnosticSessionControlResponse`, SID
//! 0x10) timing defect: server-advertised P2/P2* timing was learned but
//! never applied to the live client.
//!
//! Three defects existed together:
//! 1. The SID-0x10 timing branch lived only in `tick()`, but
//!    `on_diagnostic_message` (the `handle()` frame-arrival path) drains
//!    the same event queue first, so `tick()` never saw the event.
//! 2. `on_diagnostic_message` called `target.client.handle()` twice on the
//!    same frame - dead work that happened not to double-fire only because
//!    `complete_pending` is idempotent-by-absence.
//! 3. Even when observed, `update_timing_from_dsc` wrote
//!    `TargetState::p2`/`.p2_star`, private fields nothing ever read back
//!    into the live `UdsClient`'s `ClientConfig` - so the "learned" value
//!    was captured and discarded.
//! 4. The P2* wire value is in units of 10ms (ISO 14229-1); the ×10
//!    conversion was missing, making the applied value 10x too short.
//!
//! This test proves the fix by observing behaviour, not internal fields:
//! using the deterministic simulated clock, a request sent after a DSC
//! response with a *longer* P2 than the connection default must not time
//! out at the old deadline, and must time out at the new (correctly
//! converted) one.

use ace_core::codec::FrameWrite;
use ace_doip::header::{DoipHeader, PayloadType, ProtocolVersion};
use ace_doip::payload::{ActivationCode, RoutingActivationResponse};
use ace_gateway::tester::{DoipConnection, DoipConnectionConfig, DoipTesterEvent};
use ace_sim::clock::{Duration, Instant};

const MAX_TARGETS: usize = 4;
const TCP_MAX_FRAME: usize = 256;
const TCP_MAX_OUTBOX: usize = 8;
const MAX_CONNECTION_EVENTS: usize = 16;
const PENDING: usize = 4;
const SIM_MAX_FRAME: usize = 256;
const SIM_MAX_OUTBOX: usize = 8;
const MAX_TARGET_EVENTS: usize = 8;
const PERIODIC_DIDS: usize = 4;
const MAX_DATA: usize = 64;

type Connection = DoipConnection<
    MAX_TARGETS,
    TCP_MAX_FRAME,
    TCP_MAX_OUTBOX,
    MAX_CONNECTION_EVENTS,
    PENDING,
    SIM_MAX_FRAME,
    SIM_MAX_OUTBOX,
    MAX_TARGET_EVENTS,
    PERIODIC_DIDS,
    MAX_DATA,
>;

const ECU_ADDRESS: u16 = 0x0EF1;
const TESTER_ADDRESS: u16 = 0x0E00;

/// Default P2 the connection starts with, before any DSC response.
const DEFAULT_P2_MS: u64 = 100;

/// Server-advertised P2 in the DSC response: longer than the default, so a
/// request sent after the DSC arrives should survive past DEFAULT_P2_MS if
/// (and only if) the learned value was actually applied.
const DSC_P2_MS: u16 = 800;

/// Server-advertised P2* in units of 10ms (ISO 14229-1 wire format).
/// 400 * 10ms = 4000ms once correctly converted.
const DSC_P2EXT_10MS_UNITS: u16 = 400;
const DSC_P2EXT_EXPECTED_MS: u64 = 4_000;

fn diagnostic_message_frame(source: u16, target: u16, uds: &[u8]) -> heapless::Vec<u8, 256> {
    let mut payload: heapless::Vec<u8, 256> = heapless::Vec::new();
    payload.extend_from_slice(&source.to_be_bytes()).unwrap();
    payload.extend_from_slice(&target.to_be_bytes()).unwrap();
    payload.extend_from_slice(uds).unwrap();
    encode_frame(PayloadType::DiagnosticMessage, &payload)
}

fn routing_activation_success_frame(
    tester_address: u16,
    ecu_address: u16,
) -> heapless::Vec<u8, 256> {
    let response = RoutingActivationResponse {
        logical_address: tester_address.to_be_bytes(),
        source_address: ecu_address.to_be_bytes(),
        activation_code: ActivationCode::SuccessfullyActivated,
        reserved: [0u8; 4],
        reserved_for_oem: None,
    };

    let mut payload_buf = [0u8; 64];
    let mut cursor: &mut [u8] = &mut payload_buf;
    response.encode(&mut cursor).expect("encode should succeed");
    let written = 64 - cursor.len();

    encode_frame(
        PayloadType::RoutingActivationResponse,
        &payload_buf[..written],
    )
}

fn encode_frame(payload_type: PayloadType, payload: &[u8]) -> heapless::Vec<u8, 256> {
    let version = ProtocolVersion::Iso13400_2012;
    let header = DoipHeader {
        protocol_version: version,
        inverse_protocol_version: !(version as u8),
        payload_type,
        payload_length: payload.len() as u32,
    };

    let mut header_buf = [0u8; 8];
    let mut cursor: &mut [u8] = &mut header_buf;
    header
        .encode(&mut cursor)
        .expect("header encode should succeed");
    let header_len = 8 - cursor.len();

    let mut frame: heapless::Vec<u8, 256> = heapless::Vec::new();
    frame.extend_from_slice(&header_buf[..header_len]).unwrap();
    frame.extend_from_slice(payload).unwrap();
    frame
}

/// A DiagnosticSessionControl positive response: SID 0x50, session type
/// 0x03 (extended), the P2/P2* constants above.
fn dsc_positive_response() -> [u8; 6] {
    let p2 = DSC_P2_MS.to_be_bytes();
    let p2ext = DSC_P2EXT_10MS_UNITS.to_be_bytes();
    [0x50, 0x03, p2[0], p2[1], p2ext[0], p2ext[1]]
}

fn new_connection() -> Connection {
    let config =
        DoipConnectionConfig::new(ECU_ADDRESS).with_timing_tolerance(Duration::from_millis(0)); // verbatim, for a precise assertion
    let mut config = config;
    config.default_p2 = Duration::from_millis(DEFAULT_P2_MS);
    DoipConnection::new(TESTER_ADDRESS, config)
}

fn activate(conn: &mut Connection, now: Instant) {
    conn.on_connected(now).expect("on_connected should succeed");
    let frame = routing_activation_success_frame(TESTER_ADDRESS, ECU_ADDRESS);
    conn.handle(&frame, now)
        .expect("activation frame should be accepted");
    assert!(
        conn.is_active(),
        "connection should be Active after activation"
    );
    let _ = conn.drain_events();
}

fn drain_outbox(conn: &mut Connection) {
    let mut discard: ace_core::Vec<
        (ace_sim::io::NodeAddress, ace_core::Vec<u8, TCP_MAX_FRAME>),
        TCP_MAX_OUTBOX,
    > = ace_core::Vec::new();
    let _ = conn.drain_outbox(&mut discard);
}

fn saw_timeout(conn: &mut Connection) -> bool {
    conn.drain_events().any(|(_, _, event)| {
        matches!(
            event,
            DoipTesterEvent::Uds(ace_client::event::ClientEvent::Timeout { .. })
        )
    })
}

#[test]
fn dsc_learned_p2_is_applied_to_the_live_client() {
    let mut conn = new_connection();
    let mut now = Instant::from_micros(0);

    activate(&mut conn, now);

    // Send the DSC request and respond with a P2 far longer than the
    // connection's default - proving the *default* is not what's still in
    // effect requires that a request sent after this response survives
    // longer than DEFAULT_P2_MS.
    conn.request(ECU_ADDRESS, &[0x10, 0x03], now)
        .expect("DSC request should send once active");
    drain_outbox(&mut conn);

    let dsc = dsc_positive_response();
    let response_frame = diagnostic_message_frame(ECU_ADDRESS, TESTER_ADDRESS, &dsc);
    conn.handle(&response_frame, now)
        .expect("DSC response should be handled");
    let _ = conn.drain_events(); // discard the PositiveResponse{sid:0x10} itself

    // Send a second request (any SID other than 0x10) immediately after.
    // Its P2 deadline should now be governed by the learned value, not the
    // connection default.
    conn.request(ECU_ADDRESS, &[0x22, 0xF1, 0x90], now)
        .expect("second request should send");
    drain_outbox(&mut conn);

    // Advance past the OLD default P2 (100ms) but well short of the new
    // learned P2 (800ms). If the learned value was not applied, this tick
    // will emit Timeout for the second request - proving the defect. If it
    // was applied, no timeout should fire here.
    now = Instant::from_micros(
        now.as_micros() + Duration::from_millis(DEFAULT_P2_MS + 50).as_micros(),
    );
    conn.tick(now);
    assert!(
        !saw_timeout(&mut conn),
        "request timed out at the OLD default P2 ({DEFAULT_P2_MS}ms) - the DSC-learned P2 \
         ({DSC_P2_MS}ms) was not applied to the live client"
    );

    // Advance past the NEW learned P2 (800ms total from the second
    // request). This should now time out, proving the learned value is
    // actually governing the deadline rather than the timeout being
    // disabled entirely.
    now = Instant::from_micros(
        now.as_micros() + Duration::from_millis(u64::from(DSC_P2_MS) + 100).as_micros(),
    );
    conn.tick(now);
    assert!(
        saw_timeout(&mut conn),
        "request never timed out even after exceeding the DSC-learned P2 ({DSC_P2_MS}ms) - \
         set_timing may not be wired correctly"
    );
}

#[test]
fn dsc_p2_star_ten_millisecond_unit_conversion_is_applied() {
    // Isolates just the ×10 conversion: P2* on the wire is in units of
    // 10ms, so DSC_P2EXT_10MS_UNITS (400) must become DSC_P2EXT_EXPECTED_MS
    // (4000ms), not 400ms. Proven by extending a pending request via NRC
    // 0x78 (ResponsePending) after the DSC response, then checking it does
    // NOT expire at the unconverted (10x too short) duration.
    let mut conn = new_connection();
    let mut now = Instant::from_micros(0);

    activate(&mut conn, now);

    conn.request(ECU_ADDRESS, &[0x10, 0x03], now)
        .expect("DSC request should send once active");
    drain_outbox(&mut conn);

    let dsc = dsc_positive_response();
    let response_frame = diagnostic_message_frame(ECU_ADDRESS, TESTER_ADDRESS, &dsc);
    conn.handle(&response_frame, now)
        .expect("DSC response should be handled");
    let _ = conn.drain_events();

    conn.request(ECU_ADDRESS, &[0x2E, 0xF1, 0x90, 0x01], now)
        .expect("write request should send");
    drain_outbox(&mut conn);

    // Server sends 0x78 ResponsePending, switching this pending request to
    // the extended (P2*) deadline.
    let pending_frame = diagnostic_message_frame(ECU_ADDRESS, TESTER_ADDRESS, &[0x7F, 0x2E, 0x78]);
    conn.handle(&pending_frame, now)
        .expect("ResponsePending frame should be handled");
    let _ = conn.drain_events();

    // Advance past the WRONG (unconverted) P2* of 400ms, but short of the
    // correct 4000ms. If the ×10 conversion is missing, this fires Timeout
    // here - proving the unit bug.
    now = Instant::from_micros(
        now.as_micros() + Duration::from_millis(DSC_P2EXT_10MS_UNITS as u64 + 50).as_micros(),
    );
    conn.tick(now);
    assert!(
        !saw_timeout(&mut conn),
        "request timed out at the UNCONVERTED P2* ({DSC_P2EXT_10MS_UNITS}ms) - the ×10 \
         ISO 14229-1 unit conversion is not being applied"
    );

    // Advance past the correctly-converted P2* (4000ms) and confirm it does
    // eventually time out there.
    now = Instant::from_micros(
        now.as_micros() + Duration::from_millis(DSC_P2EXT_EXPECTED_MS).as_micros(),
    );
    conn.tick(now);
    assert!(
        saw_timeout(&mut conn),
        "request never timed out even after exceeding the correctly-converted P2* \
         ({DSC_P2EXT_EXPECTED_MS}ms)"
    );
}
