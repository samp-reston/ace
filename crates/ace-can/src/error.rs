// region: CanError

#[derive(Debug)]
pub enum CanError {
    /// The CAN ID value exceeds the valid range for its type.
    /// Standard: 0x000–0x7FF, Extended: 0x00000000–0x1FFFFFFF.
    InvalidId,
    /// The data length code exceeds the physical maximum for the frame type.
    /// Classic CAN: 0–8, CAN FD: valid DLC values mapping to 0–64.
    InvalidDlc(u8),
    /// The frame buffer is too short to contain the declared DLC worth of data.
    BufferTooShort { expected: usize, actual: usize },
    /// A flag combination in the frame is illegal per the CAN/CAN FD spec.
    /// e.g. RTR set on a CAN FD frame, BRS set without EDL.
    InvalidFlags,
    /// Underlying transport error from ace-core.
    Transport(ace_core::DiagError),
}

impl From<ace_core::DiagError> for CanError {
    fn from(e: ace_core::DiagError) -> Self {
        CanError::Transport(e)
    }
}

// endregion: CanError

// region: IsoTpError

#[derive(Debug)]
pub enum IsoTpError {
    /// A consecutive frame arrived with an unexpected sequence number.
    /// Indicates a lost or reordered frame.
    SequenceError { expected: u8, actual: u8 },
    /// A consecutive frame arrived before a first frame was received.
    /// The reassembler was not in an active session.
    UnexpectedConsecutiveFrame,
    /// A flow control frame arrived with an unknown flow status nibble.
    UnknownFlowStatus(u8),
    /// The declared message length in the first frame is zero.
    InvalidLength,
    /// The payload provided to the segmenter exceeds what ISO-TP can
    /// express in a first frame length field (classic CAN: 4095 bytes,
    /// CAN FD: 4294967295 bytes via escape sequence).
    PayloadTooLarge,
    /// A single frame was received with a length of zero.
    EmptySingleFrame,
    /// The frame buffer does not contain enough bytes for the PCI
    /// given the current addressing mode.
    FrameTooShort { actual: usize },
    /// The reassembler received a new first frame while already mid-session.
    /// The previous session is abandoned.
    SessionAborted,
    /// Underlying CAN frame error.
    Can(CanError),
    /// The PCI bytes upper nibble does not match any known ISO-TP frame type.
    UnknownFrameType(u8),
}

impl From<CanError> for IsoTpError {
    fn from(e: CanError) -> Self {
        IsoTpError::Can(e)
    }
}

impl From<ace_core::DiagError> for IsoTpError {
    fn from(e: ace_core::DiagError) -> Self {
        IsoTpError::Can(CanError::Transport(e))
    }
}

// endregion: IsoTpError
