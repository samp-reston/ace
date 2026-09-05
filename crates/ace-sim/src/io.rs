//! Re-export only. Addressing, raw messages, transport traits, and connection
//! events are foundation abstractions and live in `ace-core::io`.

pub use ace_core::io::{FrameTransport, LinkEvent, MessageTransport, NodeAddress, RawMessage};
