#[cfg(feature = "tokio")]
pub mod tokio_codec {
    extern crate alloc;
    use bytes::BytesMut;
    use tokio_util::codec::Decoder;

    use crate::{
        codec::{DecodeOutcome, FrameLimits, decode_frame},
        error::DoipValidationError,
    };

    #[derive(Debug, derive_more::Display, derive_more::From)]
    pub enum DoipCodecError {
        #[display("I/O error: {_0}")]
        #[from]
        Io(std::io::Error),

        #[display("DoIP frame invalid: {_0:?}")]
        #[from]
        Frame(DoipValidationError),
    }

    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
    pub struct DoipFrameDecoder {
        pub limits: FrameLimits,
    }

    impl Decoder for DoipFrameDecoder {
        type Item = alloc::vec::Vec<u8>;
        type Error = DoipCodecError;

        fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
            match decode_frame(src, &self.limits) {
                DecodeOutcome::NeedMoreForHeader | DecodeOutcome::NeedMorePayload { .. } => {
                    Ok(None)
                }
                DecodeOutcome::Frame { frame_len } => Ok(Some(src.split_to(frame_len).to_vec())),
                DecodeOutcome::Invalid(e) => Err(DoipCodecError::Frame(e)),
                DecodeOutcome::ConversionFailure => Err(DoipCodecError::Frame(
                    DoipValidationError::ConversionFailure,
                )),
            }
        }
    }
}
