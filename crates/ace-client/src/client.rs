// region: Imports

use crate::{SIM_MAX_FRAME, SIM_MAX_OUTBOX};
use ace_proto::{common::RawFrame, UdsFrame};
use ace_sim::{clock::Instant, io::NodeAddress};
use ace_uds::ext::UdsFrameExt;
use heapless::Vec;

use crate::{config::ClientConfig, event::ClientEvent, pending::PendingRequest, ClientError};

// endregion: Imports

// region: Capacity Constants

const MAX_EVENTS: usize = 32;
const MAX_DATA: usize = 256;

// endregion: Capacity Constants

// region: UDS Client

/// Stateful UDS tester client state machine.
///
/// Sends raw UDS request frames and emits [`ClientEvent`]s as responses arrive. The client tracks
/// only what is necessary for the protocol exchange - P2 and P2* timeouts on pending requests.
/// Session state, security state, and retry logic are entirely the caller's responsibility.
///
/// `N` - maximum number of concurrent pending requests. Defaults to 1. UDS is strictly sequential
/// in most implementations - use `UdsClient<1>` unless you have a specific need for pipelining.
pub struct UdsClient<const N: usize = 1> {
    config: ClientConfig,
    address: NodeAddress,
    server: NodeAddress,
    pending: Vec<PendingRequest, N>,
    outbox: Vec<(NodeAddress, Vec<u8, SIM_MAX_FRAME>), SIM_MAX_OUTBOX>,
    events: Vec<ClientEvent, MAX_EVENTS>,
    periodic_dids: Vec<u8, 16>,
}

impl<const N: usize> UdsClient<N> {
    pub fn new(config: ClientConfig, address: NodeAddress) -> Self {
        let server = NodeAddress(config.target_address as u32);
        Self {
            config,
            address,
            server,
            pending: Vec::new(),
            outbox: Vec::new(),
            events: Vec::new(),
            periodic_dids: Vec::new(),
        }
    }

    // region: SimNode Surface

    pub fn address(&self) -> &NodeAddress {
        &self.address
    }

    /// Delivers an inbound frame to  the client.
    ///
    /// Classifies the frame before attempting any pending request match:
    ///
    /// 1. Empty frame - ignored
    /// 2. `0x7F` - negative response - matched by requested SID at byte 1
    /// 3. First byte is a subscribed periodic DID - `PeriodicData` event
    /// 4. First byte has bit 6 set - positive response - matched by request SID
    /// 5. Anything else - `Unsolicited` event
    ///
    /// Periodic classification must precede positive response classification because a periodic
    /// DID low byte could theoretically have bit 6 set. Explicit subscription registry takes
    /// priority.
    pub fn handle(
        &mut self,
        _src: &NodeAddress,
        data: &[u8],
        now: Instant,
    ) -> Result<(), ClientError> {
        // Nothing to do for empty frames
        let first = match data.first().copied() {
            Some(b) => b,
            None => return Ok(()),
        };

        let frame = UdsFrame::from_slice(data);

        // region: Negative response [0x7F, requested_sid, nrc]

        if first == 0x7F {
            let requested_sid = data.get(1).copied().unwrap_or(0);
            let nrc = data.get(2).copied().unwrap_or(0);

            // 0x78 Response Pending - extend deadline, keep pending, emit event
            if nrc == 0x78 {
                if let Some(p) = self.pending.iter_mut().find(|p| p.sid == requested_sid) {
                    p.extend(now, self.config.p2_extended_timeout);

                    let _ = self
                        .events
                        .push(ClientEvent::ResponsePending { sid: requested_sid });
                }

                return Ok(());
            }

            self.complete_pending(requested_sid);

            let _ = self.events.push(ClientEvent::NegativeResponse {
                sid: requested_sid,
                nrc,
            });

            return Ok(());
        }

        // endregion: Negative response [0x7F, requested_sid, nrc]

        // region: Periodic Data
        //
        // [periodic_data_identifier (1 byte), data_record (n bytes)]
        // No SID prefix - first byte IS the periodic DID low byte. Only classifiable if the client
        // has a matching subscription.

        if self.periodic_dids.contains(&first) {
            // payload() gives bytes after the first byte - the data record
            let raw = frame.as_bytes();
            let record = raw.get(1..).unwrap_or(&[]);
            let mut buf: Vec<u8, MAX_DATA> = Vec::new();
            let _ = buf.extend_from_slice(&record[..record.len().min(MAX_DATA)]);

            let _ = self.events.push(ClientEvent::PeriodicData {
                did: first,
                data: buf,
            });

            return Ok(());
        }

        // endregion: Periodic Data

        // region: Positive Response - first byte has bit 6 set
        //
        // UDS positive response SID = request SID | 0x40.
        // 0x7F is already handled above so cannot reach here.
        if first & 0x40 != 0 {
            let request_sid = first & !0x40u8;

            if self.complete_pending(request_sid) {
                let payload = frame.payload();
                let mut buf: Vec<u8, MAX_DATA> = Vec::new();
                let _ = buf.extend_from_slice(&payload[..payload.len().min(MAX_DATA)]);

                let _ = self.events.push(ClientEvent::PositiveResponse {
                    sid: request_sid,
                    data: buf,
                });

                return Ok(());
            }
        }

        // endregion: Positive Response

        // region: Unsolicited
        //
        // Frame did no match any classification or had no matching pending request. Emit raw
        // bytes for observability.
        {
            let raw = frame.as_bytes();
            let mut buf: Vec<u8, MAX_DATA> = Vec::new();
            let _ = buf.extend_from_slice(&raw[..raw.len().min(MAX_DATA)]);

            let _ = self.events.push(ClientEvent::Unsolicited { data: buf });
        }

        // endregion: Unsolicited

        Ok(())
    }

    /// Advances internal timers - expires timed-out pending requests.
    pub fn tick(&mut self, now: Instant) -> Result<(), ClientError> {
        let mut expired: Vec<u8, N> = Vec::new();

        for p in self.pending.iter() {
            if p.is_expired(now) {
                let _ = expired.push(p.sid);
            }
        }

        for sid in expired {
            self.complete_pending(sid);
            let _ = self.events.push(ClientEvent::Timeout { sid });
        }

        Ok(())
    }

    /// Drains pending outbound frames into `out`.
    pub fn drain_outbox(
        &mut self,
        out: &mut Vec<(NodeAddress, Vec<u8, SIM_MAX_FRAME>), SIM_MAX_OUTBOX>,
    ) -> usize {
        let n = self.outbox.len();

        for item in self.outbox.drain(..) {
            let _ = out.push(item);
        }

        n
    }

    // endregion: SimNode Surface

    // region: Request API

    /// Enqueues a raw UDS request for transmission.
    ///
    /// `data` must begin with the SID byte followed by any parameters. The client tracks this as a
    /// pending request and starts the P2 timer.
    ///
    /// Returns `ClientError::QueueFull` if N concurrent requests are already pending.
    pub fn request(&mut self, data: &[u8], now: Instant) -> Result<(), ClientError> {
        let sid = match data.first().copied() {
            Some(b) => b,
            None => return Err(ClientError::EmptyRequest),
        };

        if self.pending.is_full() {
            return Err(ClientError::QueueFull);
        }

        let mut frame: Vec<u8, SIM_MAX_FRAME> = Vec::new();
        let _ = frame.extend_from_slice(data);

        self.outbox
            .push((self.server.clone(), frame))
            .map_err(|_| ClientError::OutboxFull)?;

        let _ = self
            .pending
            .push(PendingRequest::new(sid, now, self.config.p2_timeout));

        Ok(())
    }

    // endregion: Request API

    // region: Periodic Subscription API

    /// Registers the low byte of a periodic DID identifier as subscribed.
    ///
    /// Inbound frames whose first byte matches a subscribed DID low byte are classified as
    /// `PeriodicData` events rather than `Unsolicited`.
    ///
    /// The low byte corresponds to the `periodic_data_identifier` field in
    /// `ReadDataByPeriodicIdentifierResponseData`. For a DID of `0xF201` the low byte is `0x01`.
    ///
    /// Silently does nothing if already subscribed or the registry is full.
    pub fn subscribe_periodic(&mut self, did_low_byte: u8) {
        if !self.periodic_dids.contains(&did_low_byte) {
            let _ = self.periodic_dids.push(did_low_byte);
        }
    }

    /// Removes a periodic DID subscription.
    ///
    /// After calling this, frames with this DID low byte will be classified as `Unsolicited`
    /// rather than `PeriodicData`.
    pub fn unsubscribe_periodic(&mut self, did_low_byte: u8) {
        self.periodic_dids.retain(|&d| d != did_low_byte);
    }

    /// Returns true if the given DID low byte is currently subscribed.
    pub fn is_periodic_subscribed(&self, did_low_byte: u8) -> bool {
        self.periodic_dids.contains(&did_low_byte)
    }

    // endregion: Periodic Subscription API

    // region: Event API

    /// Drains all accumulated events, returning an iterator.
    ///
    /// Events are consumed - calling `drain_events` twice returns events on the first call and
    /// nothing on the second.
    pub fn drain_events(&mut self) -> impl Iterator<Item = ClientEvent> + '_ {
        self.events.drain(..)
    }

    /// Returns true if any events are pending.
    pub fn has_events(&self) -> bool {
        !self.events.is_empty()
    }

    /// Returns the number of currently pending requests.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    // endregion: Event API

    // region: Internal Helpers

    /// Removes a pending request by SID. Returns true if found and removed.
    fn complete_pending(&mut self, sid: u8) -> bool {
        if let Some(pos) = self.pending.iter().position(|p| p.sid == sid) {
            self.pending.remove(pos);
            true
        } else {
            false
        }
    }

    // endregion: Internal Helpers
}

// endregion: UDS Client
