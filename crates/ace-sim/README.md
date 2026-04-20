# `ace-sim`

Deterministic simulation infrastructure. Everything needed to test protocol state machines reproducibly.

**`SimBus<N, Q>`** — message delivery with fault injection. Seeded RNG makes every run reproducible. Configurable faults: message loss, reorder, delay, corruption, timeout.

**`TcpSimBus<N, Q>`** — wraps `SimBus` and adds TCP connection state. The bus owns connection state — nodes don't track it. TCP fault injection: connection refused, mid-session reset, connect timeout.

**`CanSimBus<N, Q>`** — wraps `SimBus` and adds CAN bus state (Active / ErrorPassive / BusOff). CAN fault injection: arbitration loss, bit error, bus-off.

**`SimNode<N, Q>`** — trait for nodes on the simulation buses:

```rust
pub trait SimNode<const N: usize, const Q: usize> {
    type Error: core::fmt::Debug;
    fn address(&self) -> &NodeAddress;
    fn handle(&mut self, src: &NodeAddress, data: &[u8], now: Instant) -> Result<(), Self::Error>;
    fn tick(&mut self, now: Instant) -> Result<(), Self::Error>;
    fn drain_outbox(&mut self, out: &mut Vec<(NodeAddress, Vec<u8, N>), Q>) -> usize;
}
```

**`SimNodeErased<N, Q>`** — object-safe version with errors swallowed internally, enabling heterogeneous slices of different node types.

**`SimRunner<N, Q>`** — drives `SimNodeErased` slices over a `SimBus`.

**`TcpSimRunner<N, Q>`** — drives nodes over a `TcpSimBus`, additionally delivering `TcpEvent`s to nodes implementing `TcpEventHandler`.

**`CanSimRunner<N, Q>`** — drives nodes over a `CanSimBus`, additionally delivering `CanEvent`s to nodes implementing `CanEventHandler`.

**`FaultConfig`** — three presets: `none()`, `light()`, `chaos()`. All probabilities are `(numerator, denominator)` pairs for reproducibility.
