// region: Imports

use heapless::Vec;

// endregion: Imports

// region: Client Event

/// An observable event emitted by the UDS client.
///
/// Events accumulate in the client's event queue across ticks and are drained by the caller via
/// `drain_events()`. Each event corresponds to a completed or failed request exchange.
///
/// The caller decides what to do with each event - the client never retries, escalates, or takes
/// corrective action autonomously.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientEvent {
    /// A positive response was received for the given service.
    ///
    /// `sid` is the request SID (not the response SID).
    /// `data` contains the response payload bytes after the response SID.
    PositiveResponse { sid: u8, data: Vec<u8, 256> },

    /// A negative response was received.
    ///
    /// `sid` is the request SID.
    /// `nrc` is the Negative Response Code byte.
    NegativeResponse { sid: u8, nrc: u8 },

    /// A `0x78` Response Pending NRC was received.
    ///
    /// The client has switched to the P2* extended timeout and is still waiting for the final
    /// response. This event is emitted once per `0x78` received - the caller can observe how many
    /// were received before the final response.
    ResponsePending { sid: u8 },

    /// No response was received within the P2 or P2* timeout.
    ///
    /// The pending request has been discarded. The caller must decide whether to retry.
    Timeout { sid: u8 },

    /// A periodic data response arrived from the server.
    ///
    /// `[periodic_data_identifier (1 byte), data_record (n bytes)]`
    ///
    /// No SID prefix is present - `did` is the first byte of the frame, which is the low byte of
    /// the periodic DID identifier. `data` contains the data record bytes that followed.
    ///
    /// Only emitted for DID low bytes registered via `subscribe_periodic`
    PeriodicData {
        /// Low byte of the periodic DID identifier.
        did: u8,

        /// Data record bytes
        data: Vec<u8, 256>,
    },

    /// A response arrived with no matching pending request.
    ///
    /// This can occur under fault injection (reordered or delayed responses arriving after a
    /// timeout) or if the server sends an unsolicited response.
    Unsolicited { data: Vec<u8, 256> },
}

// endregion: Client Event
