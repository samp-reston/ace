use ace_core::codec::FrameRead;
use ace_proto::common::{RawFrame, RawFrameMut};
use ace_proto::uds::{UdsFrame, UdsFrameMut};

use crate::constants::{
    MIN_FRAME_LEN, MIN_SUB_FUNCTION_FRAME_LEN, NEGATIVE_RESPONSE_NRC_OFFSET,
    NEGATIVE_RESPONSE_REQUESTED_SID_OFFSET, SUB_FUNCTION_OFFSET, SUB_FUNCTION_VALUE_MASK,
    SUPPRESS_POSITIVE_RESPONSE_MASK,
};
use crate::error::{UdsError, ValidationError};
use crate::message::ServiceIdentifier;
use crate::message::UdsMessage;
use crate::message::{decode_message, NegativeResponseCode};

// region: UdsFrameExt

/// Semantic extension trait for [`UdsFrame`].
///
/// Provides protocol-level interpretation of the raw bytes in a `UdsFrame` -
/// service identification, sub-function parsing, response classification,
/// payload access, and message conversion.
///
/// Implemented for [`UdsFrame`] in `ace-uds`. The raw frame type in
/// `ace-proto` carries no protocol knowledge - all UDS semantics are
/// provided here.
pub trait UdsFrameExt<'a> {
    // region: Service Identification

    /// Returns the `ServiceIdentifier` for this frame if the first byte
    /// corresponds to a known UDS service.
    ///
    /// Returns `None` if the frame is empty or the first byte does not
    /// match a known service identifier. Per ISO 14229, an absent SID
    /// implies a periodic data response.
    #[must_use]
    fn service_identifier(&self) -> Option<ServiceIdentifier>;

    // endregion: Service Identification

    // region: Payload Access

    /// Returns the payload bytes of the frame, excluding the SID byte.
    ///
    /// If no service identifier is present the entire buffer is returned,
    /// consistent with the periodic data response convention.
    #[must_use]
    fn payload(&self) -> &[u8];

    /// Returns an iterator over the payload bytes, excluding the SID byte.
    fn data_iter(&self) -> core::slice::Iter<'_, u8>;

    // endregion: Payload Access

    // region: Sub-Function

    /// Returns the raw sub-function byte for services that carry one,
    /// including the suppress positive response bit.
    ///
    /// Returns `None` for services that do not define a sub-function byte,
    /// or if the frame is too short to contain one.
    #[must_use]
    fn sub_function(&self) -> Option<u8>;

    /// Returns the sub-function value with the suppress positive response
    /// bit masked off.
    #[must_use]
    fn sub_function_value(&self) -> Option<u8> {
        self.sub_function().map(|sf| sf & SUB_FUNCTION_VALUE_MASK)
    }

    /// Returns `true` if the suppress positive response bit is set in the
    /// sub-function byte (`bit 7 = 1`).
    ///
    /// Always returns `false` if the service does not define a sub-function.
    #[must_use]
    fn is_suppressed(&self) -> bool;

    // endregion: Sub-Function

    // region: Response Classification

    /// Returns `true` if this frame is a positive response.
    ///
    /// A frame is a positive response if it is not a negative response -
    /// i.e. the SID is not `0x7F`.
    #[must_use]
    fn is_positive_response(&self) -> bool {
        !self.is_negative_response()
    }

    /// Returns `true` if this frame is a negative response (SID `0x7F`).
    #[must_use]
    fn is_negative_response(&self) -> bool;

    /// Returns the `NegativeResponseCode` if this is a negative response.
    ///
    /// Negative response format: `[0x7F, RequestedSID, NRC]`
    ///
    /// Returns `None` if this is not a negative response or the frame
    /// is too short to contain an NRC byte.
    #[must_use]
    fn negative_response_code(&self) -> Option<NegativeResponseCode>;

    /// Returns the requested `ServiceIdentifier` from a negative response.
    ///
    /// Negative response format: `[0x7F, RequestedSID, NRC]`
    ///
    /// Returns `None` if this is not a negative response or the SID byte
    /// at position 1 is not a known service identifier.
    #[must_use]
    fn requested_service_identifier(&self) -> Option<ServiceIdentifier>;

    // endregion: Response Classification

    // region: Validation

    /// Validates the frame against UDS protocol rules.
    ///
    /// Checks that the SID is known, the payload length is appropriate
    /// for the service, and the sub-function (if present) is valid.
    ///
    /// Individual service implementations may provide deeper validation
    /// via their own `validate()` methods.
    fn validate(&self) -> Result<(), UdsError>;

    /// Returns `true` if [`validate`](UdsFrameExt::validate) succeeds.
    #[must_use]
    #[inline]
    fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }

    // endregion: Validation

    // region: Conversion

    /// Parses this frame into a structured [`UdsMessage`].
    fn to_message(&'a self) -> Result<UdsMessage<'a>, UdsError>;

    // endregion: Conversion
}

// endregion: UdsFrameExt

// region: UdsFrameMutExt

/// Semantic extension trait for [`UdsFrameMut`].
///
/// Extends [`UdsFrameExt`] with mutation methods that require protocol
/// knowledge - suppress/un-suppress operates on the sub-function byte
/// whose position is service-dependent.
pub trait UdsFrameMutExt<'a>: UdsFrameExt<'a> {
    /// Sets the suppress positive response bit in the sub-function byte.
    ///
    /// Has no effect if the service does not define a sub-function, or
    /// if the bit is already set.
    fn suppress(&mut self);

    /// Clears the suppress positive response bit in the sub-function byte.
    ///
    /// Has no effect if the service does not define a sub-function, or
    /// if the bit is already clear.
    fn un_suppress(&mut self);
}

// endregion: UdsFrameMutExt

// region: UdsFrameExt impl for UdsFrame

impl<'a> UdsFrameExt<'a> for UdsFrame<'a> {
    // region: Service Identification

    fn service_identifier(&self) -> Option<ServiceIdentifier> {
        let byte = self.as_bytes().first()?;
        let mut buf = core::slice::from_ref(byte);
        ServiceIdentifier::decode(&mut buf).ok()
    }

    // endregion: Service Identification

    // region: Payload Access

    fn payload(&self) -> &[u8] {
        match self.service_identifier() {
            Some(_) => &self.as_bytes()[MIN_FRAME_LEN..],
            None => self.as_bytes(),
        }
    }

    fn data_iter(&self) -> core::slice::Iter<'_, u8> {
        self.payload().iter()
    }

    // endregion: Payload Access

    // region: Sub-Function

    fn sub_function(&self) -> Option<u8> {
        let sid = self.service_identifier()?;
        if !sid.has_sub_function() {
            return None;
        }
        self.as_bytes().get(SUB_FUNCTION_OFFSET).copied()
    }

    fn is_suppressed(&self) -> bool {
        match self.sub_function() {
            Some(sf) => sf & SUPPRESS_POSITIVE_RESPONSE_MASK != 0,
            None => false,
        }
    }

    // endregion: Sub-Function

    // region: Response Classification

    fn is_negative_response(&self) -> bool {
        self.service_identifier() == Some(ServiceIdentifier::NegativeResponse)
    }

    fn negative_response_code(&self) -> Option<NegativeResponseCode> {
        if !self.is_negative_response() {
            return None;
        }
        let byte = self.as_bytes().get(NEGATIVE_RESPONSE_NRC_OFFSET)?;
        let mut buf = core::slice::from_ref(byte);
        NegativeResponseCode::decode(&mut buf).ok()
    }

    fn requested_service_identifier(&self) -> Option<ServiceIdentifier> {
        if !self.is_negative_response() {
            return None;
        }
        let byte = self
            .as_bytes()
            .get(NEGATIVE_RESPONSE_REQUESTED_SID_OFFSET)?;
        let mut buf = core::slice::from_ref(byte);
        ServiceIdentifier::decode(&mut buf).ok()
    }

    // endregion: Response Classification

    // region: Validation

    fn validate(&self) -> Result<(), UdsError> {
        // Frame must have at least a SID byte
        if self.as_bytes().is_empty() {
            return Err(ValidationError::InvalidLength {
                expected: MIN_FRAME_LEN,
                actual: 0,
            }
            .into());
        }

        // SID must be a known service identifier
        let sid = self
            .service_identifier()
            .ok_or_else(|| ValidationError::UnsupportedService(self.as_bytes()[0]))?;

        // If the service has a sub-function the frame must be long enough
        // to contain it
        if sid.has_sub_function() && self.as_bytes().len() < MIN_SUB_FUNCTION_FRAME_LEN {
            return Err(ValidationError::InvalidLength {
                expected: MIN_SUB_FUNCTION_FRAME_LEN,
                actual: self.as_bytes().len(),
            }
            .into());
        }

        Ok(())
    }

    // endregion: Validation

    // region: Conversion

    fn to_message(&'a self) -> Result<UdsMessage<'a>, UdsError> {
        decode_message(self.as_bytes())
    }

    // endregion: Conversion
}

// endregion: UdsFrameExt impl for UdsFrame

// region: UdsFrameExt impl for UdsFrameMut

/// `UdsFrameMut` delegates all read accessors to its immutable counterpart,
/// keeping the semantic logic in one place.
impl<'a> UdsFrameExt<'a> for UdsFrameMut<'a> {
    fn service_identifier(&self) -> Option<ServiceIdentifier> {
        UdsFrame::from_slice(self.as_bytes()).service_identifier()
    }

    fn payload(&self) -> &[u8] {
        match self.service_identifier() {
            Some(_) => &self.as_bytes()[MIN_FRAME_LEN..],
            None => self.as_bytes(),
        }
    }

    fn data_iter(&self) -> core::slice::Iter<'_, u8> {
        self.payload().iter()
    }

    fn sub_function(&self) -> Option<u8> {
        UdsFrame::from_slice(self.as_bytes()).sub_function()
    }

    fn is_suppressed(&self) -> bool {
        UdsFrame::from_slice(self.as_bytes()).is_suppressed()
    }

    fn is_negative_response(&self) -> bool {
        UdsFrame::from_slice(self.as_bytes()).is_negative_response()
    }

    fn negative_response_code(&self) -> Option<NegativeResponseCode> {
        UdsFrame::from_slice(self.as_bytes()).negative_response_code()
    }

    fn requested_service_identifier(&self) -> Option<ServiceIdentifier> {
        UdsFrame::from_slice(self.as_bytes()).requested_service_identifier()
    }

    fn validate(&self) -> Result<(), UdsError> {
        UdsFrame::from_slice(self.as_bytes()).validate()
    }

    fn to_message(&'a self) -> Result<UdsMessage<'a>, UdsError> {
        decode_message(self.as_bytes())
    }
}

// endregion: UdsFrameExt impl for UdsFrameMut

// region: UdsFrameMutExt impl for UdsFrameMut

impl<'a> UdsFrameMutExt<'a> for UdsFrameMut<'a> {
    fn suppress(&mut self) {
        if self.is_suppressed() || self.sub_function().is_none() {
            return;
        }
        if let Some(sf) = self.as_bytes_mut().get_mut(SUB_FUNCTION_OFFSET) {
            *sf |= SUPPRESS_POSITIVE_RESPONSE_MASK;
        }
    }

    fn un_suppress(&mut self) {
        if !self.is_suppressed() || self.sub_function().is_none() {
            return;
        }
        if let Some(sf) = self.as_bytes_mut().get_mut(SUB_FUNCTION_OFFSET) {
            *sf &= SUB_FUNCTION_VALUE_MASK;
        }
    }
}

// endregion: UdsFrameMutExt impl for UdsFrameMut
