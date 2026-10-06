use crate::UdsError;
use ace_macros::FrameCodec;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct WriteMemoryByAddressRequest<'a> {
    pub address_and_length_format_identifier: u8,
    #[frame(
        length = "(address_and_length_format_identifier & 0x0F) as usize",
        bytes
    )]
    pub memory_address: &'a [u8],
    #[frame(length = "(address_and_length_format_identifier >> 4) as usize", bytes)]
    pub memory_size: &'a [u8],
    #[frame(read_all, bytes)]
    pub data_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct WriteMemoryByAddressResponse<'a> {
    pub address_and_length_format_identifier: u8,
    #[frame(
        length = "(address_and_length_format_identifier & 0x0F) as usize",
        bytes
    )]
    pub memory_address: &'a [u8],
    #[frame(length = "(address_and_length_format_identifier >> 4) as usize", bytes)]
    pub memory_size: &'a [u8],
}

#[cfg(test)]
mod tests {
    use super::*;
    use ace_core::codec::{FrameWrite, decode_from_slice};

    #[cfg(feature = "alloc")]
    use alloc::borrow::ToOwned;
    #[cfg(feature = "alloc")]
    use alloc::vec;

    // address_and_length_format_identifier = 0x12:
    //   low nibble  = 2 → memory_address is 2 bytes
    //   high nibble = 1 → memory_size    is 1 byte
    //   remainder        → data_record
    const REQUEST_FRAME: &[u8] = &[0x12, 0xAA, 0xBB, 0x04, 0xDE, 0xAD, 0xBE, 0xEF];

    // address_and_length_format_identifier = 0x12:
    //   low nibble  = 2 → memory_address is 2 bytes
    //   high nibble = 1 → memory_size    is 1 byte
    const RESPONSE_FRAME: &[u8] = &[0x12, 0xAA, 0xBB, 0x04];

    #[test]
    fn request_decode() {
        let req: WriteMemoryByAddressRequest = decode_from_slice(REQUEST_FRAME).unwrap();
        assert_eq!(req.address_and_length_format_identifier, 0x12);
        assert_eq!(req.memory_address, &[0xAA, 0xBB]);
        assert_eq!(req.memory_size, &[0x04]);
        assert_eq!(req.data_record, &[0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn request_encode_roundtrip() {
        let req: WriteMemoryByAddressRequest = decode_from_slice(REQUEST_FRAME).unwrap();
        let mut buf = [0u8; 64];
        req.encode(&mut buf.as_mut()).unwrap();
        assert_eq!(&buf[..REQUEST_FRAME.len()], REQUEST_FRAME);
    }

    #[test]
    fn response_decode() {
        let resp: WriteMemoryByAddressResponse = decode_from_slice(RESPONSE_FRAME).unwrap();
        assert_eq!(resp.address_and_length_format_identifier, 0x12);
        assert_eq!(resp.memory_address, &[0xAA, 0xBB]);
        assert_eq!(resp.memory_size, &[0x04]);
    }

    #[test]
    fn response_encode_roundtrip() {
        let resp: WriteMemoryByAddressResponse = decode_from_slice(RESPONSE_FRAME).unwrap();
        let mut buf = [0u8; 64];
        resp.encode(&mut buf.as_mut()).unwrap();
        assert_eq!(&buf[..RESPONSE_FRAME.len()], RESPONSE_FRAME);
    }

    #[test]
    fn request_truncated_address() {
        // claims 3 address bytes but only 2 present
        let bad: &[u8] = &[0x13, 0xAA, 0xBB];
        let result = decode_from_slice::<WriteMemoryByAddressRequest>(bad);
        assert!(result.is_err());
    }

    #[test]
    fn request_truncated_size() {
        // claims 2 size bytes but only 1 present after address
        let bad: &[u8] = &[0x22, 0xAA, 0xBB, 0x04];
        let result = decode_from_slice::<WriteMemoryByAddressRequest>(bad);
        assert!(result.is_err());
    }

    #[test]
    fn response_truncated() {
        // claims 2 address bytes but only 1 present
        let bad: &[u8] = &[0x12, 0xAA];
        let result = decode_from_slice::<WriteMemoryByAddressResponse>(bad);
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn request_to_owned() {
        let req: WriteMemoryByAddressRequest = decode_from_slice(REQUEST_FRAME).unwrap();
        let owned = req.to_owned();
        assert_eq!(owned.address_and_length_format_identifier, 0x12);
        assert_eq!(owned.memory_address, vec![0xAA, 0xBB]);
        assert_eq!(owned.memory_size, vec![0x04]);
        assert_eq!(owned.data_record, vec![0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn response_to_owned() {
        let resp: WriteMemoryByAddressResponse = decode_from_slice(RESPONSE_FRAME).unwrap();
        let owned = resp.to_owned();
        assert_eq!(owned.address_and_length_format_identifier, 0x12);
        assert_eq!(owned.memory_address, vec![0xAA, 0xBB]);
        assert_eq!(owned.memory_size, vec![0x04]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn owned_request_encode_roundtrip() {
        let owned: WriteMemoryByAddressRequestOwned = decode_from_slice(REQUEST_FRAME).unwrap();
        let mut buf = [0u8; 64];
        owned.encode(&mut buf.as_mut()).unwrap();
        assert_eq!(&buf[..REQUEST_FRAME.len()], REQUEST_FRAME);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn owned_response_encode_roundtrip() {
        let owned: WriteMemoryByAddressResponseOwned = decode_from_slice(RESPONSE_FRAME).unwrap();
        let mut buf = [0u8; 64];
        owned.encode(&mut buf.as_mut()).unwrap();
        assert_eq!(&buf[..RESPONSE_FRAME.len()], RESPONSE_FRAME);
    }
}
