# `ace-gateway`

DoIP gateway, ISO-TP bridge node, and DoIP tester.

**`DoipGateway<A, MAX_TESTERS, BUF>`** — gateway state machine. Translates DoIP frames from TCP into UDS bytes on CAN, and CAN responses back into DoIP frames. Has two faces — `handle_tcp` and `handle_can` — because it spans two buses. Routing table maps DoIP logical addresses to CAN IDs.

**`IsoTpNode<N>`** — bridges raw UDS bytes to ISO-TP CAN frames. Two independent segmenters (request and response directions) to handle concurrent multi-frame exchanges. Key methods: `handle_from_gateway(uds_data)`, `handle_uds_response(uds_data)`, `handle_from_ecu(can_frame)`.

**`DoipTester<MAX_CONNECTIONS, MAX_TARGETS>`** — models a physical DoIP tester device. Owns multiple `DoipConnection`s (one per TCP connection). Each connection addresses multiple ECUs simultaneously via `target_address`. P2/P2* timeouts are learned dynamically from `DiagnosticSessionControlResponse`. Events are tagged `(ConnectionId, TargetId, DoipTesterEvent)`.

```rust
let mut tester = DoipTester::<4, 8>::new(0x0E00, NodeAddress(0x0E00));

// Open a connection to a gateway
let conn = tester.open_connection(DoipConnectionConfig::new(0x0E80))?;

// After TCP connects (TcpSimBus::connect or real TcpStream):
// tester.on_tcp_event(&TcpEvent::ConnectionEstablished { .. }, now);
// → automatically sends RoutingActivationRequest

// Send UDS to ECU 0x0001 on that connection
tester.request(conn, 0x0001, &[0x10, 0x03], now)?;

// Drain events
for (conn_id, target_id, event) in tester.drain_events() {
    // ...
}
```

Node profiles accumulate from UDP announcements:

```rust
if let Some(profile) = tester.profile(0x0E80) {
    println!("VIN: {:?}", profile.vin);
}
```

**`GatewayConfig`** builder:

```rust
let config = GatewayConfig::new(0x0E80)
    .with_tester(0x0E00)
    .with_node(CanNodeEntry {
        logical_address:   0x0001,
        request_can_id:    0x7E0,
        response_can_id:   0x7E8,
        functional_can_id: 0x7DF,
    });
```
