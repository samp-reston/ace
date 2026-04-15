// region: Imports

use crate::nrc::NrcError;

// endregion: Imports

// region: ServerHandler

/// Application-level hook trait for UDS service handling.
///
/// The server state machine calls these hooks when a valid, session-permitted, security-cleared
/// request arrives that requires application data or action. The server handles all protocol
/// framing, session management, security access state, and periodic scheduling internally - the
/// handler only sees the decoded parameters.
///
/// # Required vs Optional Hooks
///
/// Required hooks have no default implementation - the compiler enforces them. Optional hooks
/// default to NRC 0x11 (Service Not Supported). Override only the services your ECU actually
/// supports.
///
/// # Buffer Convention
///
/// Response data is written into the provided `buf` slice. The return value is the number of valid
/// bytes written. The server sends only `buf[..len]`.
///
/// # Error Mapping
///
/// `type Error` must implement [`NrcError`] + `Into<u8>`. The server converts handler errors
/// directly to NRC bytes in the negative response.
pub trait ServerHandler {
    type Error: NrcError;

    // region: Required Hooks

    /// Reads the value of a data identifier into `buf`.
    ///
    /// Called for `ReadDataByIdentifier` (0x22) and periodic scheduling (0x2A). Returns the number
    /// of bytes written into `buf`.
    fn read_did(&self, did: u16, buf: &mut [u8]) -> Result<usize, Self::Error>;

    /// Writes a value to a data identifier.
    ///
    /// Called for `WriteDataByIdentifier` (0x2E).
    fn write_did(&mut self, did: u16, data: &[u8]) -> Result<(), Self::Error>;

    /// Executes an ECU Reset.
    ///
    /// Called for `EcuReset` (0x11). Reset Types: 0x01 Hard Reset, 0x02 KeyOffOnReset, 0x03
    /// SoftReset. The positive response is sent before this hook is called.
    fn ecu_reset(&mut self, reset_type: u8) -> Result<(), Self::Error>;

    // endregion: Required Hooks

    // region: Optional Hooks

    /// Executes a routine control operation.
    ///
    /// Called for `Routine Control` (0x31). Sub-Functions: 0x01 Start Routine, 0x02 Stop Routine,
    /// 0x03 Request Routine Results.
    ///
    /// Return the number of bytes written into `buf`.
    fn routine_control(
        &mut self,
        _routine_id: u16,
        _sub_function: u8,
        _data: &[u8],
        _buf: &mut [u8],
    ) -> Result<usize, Self::Error> {
        Err(Self::Error::service_not_supported())
    }

    /// Controls communication on a network channel.
    ///
    /// Called for `CommunicationControl` (0x28)
    fn communication_control(
        &mut self,
        _control_type: u8,
        _comm_type: u8,
    ) -> Result<usize, Self::Error> {
        Err(Self::Error::service_not_supported())
    }

    /// Initiates a data download session.
    ///
    /// Called for `RequestDownload` (0x34).
    ///
    /// Returns max block length encoded in `buf`.
    fn request_download(
        &mut self,
        _memory_address: &[u8],
        _memory_size: &[u8],
        _compression_method: u8,
        _encrypting_method: u8,
        _buf: &mut [u8],
    ) -> Result<usize, Self::Error> {
        Err(Self::Error::service_not_supported())
    }

    /// Controls an input or output signal
    ///
    /// Called for `InputOutputControlByIdentifier` (0x2F).
    ///
    /// Returns the number of bytes written into `buf`.
    fn io_control(
        &mut self,
        _did: u16,
        _parameter: u8,
        _control_state: &[u8],
        _buf: &mut [u8],
    ) -> Result<usize, Self::Error> {
        Err(Self::Error::service_not_supported())
    }

    /// Transfers a block of data.
    ///
    /// Called for `TransferData` (0x36).
    ///
    /// Returns the number of bytes written into `buf`.
    fn transfer_data(
        &mut self,
        _block_sequence_counter: u8,
        _data: &[u8],
        _buf: &mut [u8],
    ) -> Result<usize, Self::Error> {
        Err(Self::Error::service_not_supported())
    }

    /// Finalises a data transfer session.
    ///
    /// Called for `RequestTransferExit` (0x37).
    ///
    /// Returns the number of bytes written into `buf`.
    fn request_transfer_exit(
        &mut self,
        _parameter_record: &[u8],
        _buf: &mut [u8],
    ) -> Result<usize, Self::Error> {
        Err(Self::Error::service_not_supported())
    }

    /// Initiates a file transfer operation.
    ///
    /// Called for `RequestFileTransfer` (0x38).
    ///
    /// Returns the number of bytes written into `buf`.
    fn request_file_transfer(
        &mut self,
        _operation: u8,
        _path: &[u8],
        _buf: &mut [u8],
    ) -> Result<usize, Self::Error> {
        Err(Self::Error::service_not_supported())
    }

    // endregion: Optional Hooks
}

// endregion: ServerHandler
