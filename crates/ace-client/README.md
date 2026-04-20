# `ace-client`

UDS tester client state machine (ISO 14229-1 tester side).

A dumb request/response pipe. Sends raw UDS frames, emits `ClientEvent`s as responses arrive. Tracks P2/P2* timeouts per pending request. Session state, security state, and retry logic are the caller's responsibility.

```rust
let mut client = UdsClient::<1>::new(config, address);

// Send a request
client.request(&[0x10, 0x03], now)?;

// After ticking, drain events
for event in client.drain_events() {
    match event {
        ClientEvent::PositiveResponse { sid, data } => { /* ... */ }
        ClientEvent::NegativeResponse { sid, nrc }  => { /* ... */ }
        ClientEvent::ResponsePending  { sid }        => { /* extended timeout active */ }
        ClientEvent::Timeout          { sid }        => { /* no response in time */ }
        ClientEvent::PeriodicData     { did, data }  => { /* periodic DID data */ }
        ClientEvent::Unsolicited      { data }        => { /* unmatched frame */ }
    }
}
```

Periodic DID subscriptions control classification of `0xF2xx` response frames:

```rust
client.subscribe_periodic(0x90);   // DID 0xF290 low byte
client.unsubscribe_periodic(0x90);
```
