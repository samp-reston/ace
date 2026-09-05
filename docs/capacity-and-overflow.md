# Capacity and overflow

Every collection in `ace` is bounded by a const generic. This document states what
each bound means, what happens when one is reached, and how to size them.

It exists because "bounded memory with explicit errors" is the suite's central claim,
and before Phase 3 that claim was not true in the configuration most consumers build
in. See `ADR-004` for the error taxonomy this builds on.

## `N` means two different things

`ace_core::storage::Vec<T, N>` is one type with two backings, chosen by the `alloc`
feature. The bound is enforced identically either way, but what it *costs* differs:

| | without `alloc` | with `alloc` |
|---|---|---|
| Backing | `heapless::Vec<T, N>`, inline | `alloc::vec::Vec<T>`, heap |
| `N` means | **how much space is reserved** | **how much growth is permitted** |
| Static footprint | `N * size_of::<T>()` | pointer, length, capacity - independent of `N` |
| At construction | the whole buffer exists | nothing is allocated |
| Growth | none; the buffer is the buffer | on demand, never past `N` |

The consequence for sizing: on bare metal `N` is a memory budget you pay for whether
you use it or not, so it wants to be tight. In a hosted build `N` is a ceiling you pay
for only as far as you fill it, so it wants to be generous enough that legitimate
traffic never trips it.

This is asserted, not just described - `storage.rs` tests that the alloc-mode size is
independent of `N` and that nothing is allocated before first use, and that the
no-alloc size does scale with `N`. A regression that turned the bound into a static
reservation would fail those tests.

## Overflow policies

Four policies, chosen by what silent failure would actually do rather than by call
shape. Anything that can lose protocol data reports; only structurally impossible
failures are left unreported.

### 1. Propagate

The default for library code. The operation fails, the caller decides.

Used for outbound queues (`outbox`, `can_outbox`, `tcp_outbox`, `uds_outbox`), the
`ConnectionState` action queue, the client and tester event queues, and frame
assembly. The error names the resource: `OutboxFull`, `FrameTooLong`,
`ActionQueueFull`, `EventQueueFull`, `TooManyTargets`, `TooManySubscriptions`.

### 2. Answer with a negative response

Where the protocol defines an answer, the protocol answer wins over an internal error.

- A handler returning more data than the response frame can carry yields **NRC 0x14
  responseTooLong**. Note the frame needs `payload.len() + 1`: the response SID leaves
  `MAX_FRAME - 1` bytes for data, so a handler that fills a `MAX_FRAME` buffer
  completely will be refused.
- A periodic subscription the scheduler cannot store yields **NRC 0x31
  requestOutOfRange**, rather than a positive response promising data that will never
  arrive.
- Security bookkeeping that cannot be recorded yields **NRC 0x22 conditionsNotCorrect**
  and denies. This fails *closed*: `failed_count` returns 0 for an untracked level and
  `is_locked` returns false, so an unrecorded failure would mean the attempt limit
  never trips.

### 3. Fail loudly (harness only)

`ace-sim` and `ace-tests` panic. A dropped bus event or lost outbox frame would let a
deterministic run complete - just not the run that was configured, with no signal that
it diverged. Failing the test is strictly better than silently changing what it
observes.

### 4. Documented as impossible

A few pushes cannot fail by construction and say so, with a `debug_assert` where the
precondition is checkable:

- the response SID byte in `pos`, and the three bytes of a negative response - a
  negative response deliberately does not recurse into the too-long path, because a
  failed NRC has no NRC to report it
- `out.push` inside every `drain_outbox`, which is preceded by a capacity check
- `collect_due`, whose output shares `MAX_PERIODIC` with its source

Configuration builders are the one place a drop is tolerated: declaring more entries
than the bound allows is a programming error, the entry is simply absent at runtime,
and dependent services are then refused. Those carry a `debug_assert` naming the
constant.

## The drain contract

`drain_outbox` is **all-or-nothing**: if `out` cannot hold everything queued, nothing
is moved and `Err` is returned, so the caller can retry with a clear buffer. It refuses
before draining rather than during, because the source is emptied as it is read and a
partial move would lose the remainder unrecoverably.

The returned count is therefore exact. It previously was not: the count was captured
before the pushes and the pushes were discarded, so it reported frames that may never
have left the node.

Note that `out` is typed `&mut Vec<_, MAX_OUTBOX>` - the same bound as the source - so
the type system already rules out passing a smaller buffer. The remaining hazard is
reusing a buffer that still holds earlier frames.

## Sizing the bounds

| Collection | Bound | Overflow |
|---|---|---|
| `UdsServer::outbox` | `MAX_OUTBOX` | `ServerError::OutboxFull` |
| response frame | `MAX_FRAME` | NRC 0x14 |
| `SecurityState::failed_attempts` / `lockout_until` | `MAX_SECURITY_LEVELS` | NRC 0x22, denies |
| `SecurityState::pending_seed` | `MAX_SEED` | NRC 0x14 |
| `PeriodicState::entries` | `MAX_PERIODIC` | NRC 0x31 |
| `UdsClient::pending` | `PENDING` | `ClientError::QueueFull` |
| `UdsClient::events` | `MAX_TARGET_EVENTS` | `ClientError::EventQueueFull` |
| `UdsClient::periodic_dids` | `PERIODIC_DIDS` | `ClientError::TooManySubscriptions` |
| `ConnectionState::actions` | `CONNECTION_MAX_ACTIONS` | `DoipError::ActionQueueFull` |
| `DoipGateway::connections` | `MAX_CONNECTIONS` | `GatewayError::NoConnectionSlot` |
| `DoipGateway::tcp_outbox` / `can_outbox` | `TCP_MAX_OUTBOX` / `CAN_MAX_OUTBOX` | `GatewayError::*OutboxFull` |
| `DoipConnection::targets` | `MAX_TARGETS` | `DoipTesterError::TooManyTargets` |
| `IsoTpNode::can_outbox` / `uds_outbox` | `ISOTP_MAX_OUT` / `UDS_MAX_OUTBOX` | `IsoTpNodeError::OutboxFull` |
| `CanSimBus::events` | `CAN_MAX_EVENTS` | panics (harness) |
| `TcpSimBus::events` | `TCP_MAX_EVENTS` | panics (harness) |
| `ServerConfig` / `GatewayConfig` tables | `MAX_*` | dropped, `debug_assert` |

Two bounds are worth extra care because the failure is not local to the operation:

- **`MAX_SECURITY_LEVELS`** must cover every level you configure. ISO 14229-1 permits
  far more than a handful, and this bound sizes the attempt and lockout tables as well
  as the config table.
- **`CONNECTION_MAX_ACTIONS`** must cover the actions one frame can produce. Because a
  full queue now fails the operation, an undersized bound takes a live connection down
  rather than degrading quietly.

## Known gaps

- `ace-can`'s ISO-TP segmenter and parts of `ace-doip` use `heapless::Vec` directly
  rather than `ace_core::storage::Vec`. Their bounds are still enforced, but they do not
  share the newtype's contract.
- `ace-transport`'s `DoipVehicleDriver` runs the tester on its own thread with no
  channel back to the caller, so a handful of results there are discarded by necessity
  rather than by policy.
