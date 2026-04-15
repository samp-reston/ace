// region: Imports

use ace_sim::clock::Duration;

// endregion: Imports

/// DST property tests for ReadDataByIdentifier (0x22) and WriteDataByIdentifier (0x2E).
///
/// Properties:
///     - P1: RDBI returns the expected data for a configured DID.
///     - P2: RDBI for an unknown DID returns NRC RequestOutOfRange.
///     - P3: RDBI in wrong session returns NRC ServiceNotSupportedInActiveSession.
///     - P4: WDBI writes data and subsequent RDBI returns the written value.
///     - P5: WDBI requires security access - returns NRC SecurityAccessDenied without it.
pub mod data;

/// DST property tests for ReadDataByPeriodicIdentifier (0x2A).
///
/// Properties:
///     - P1: After subscribing fast rate, client receives PeriodicData events.
///     - P2: After StopSending, client receives no further PeriodicData events.
///     - P3: Periodic data carries the correct DID low byte and data.
pub mod periodic;

/// DST property tests for SecurityAccess (0x27).
///
/// Properties tested:
///     - P1: Full seed/key exchange unlocks security level on server.
///     - P2: Wrong key increments failed attempts.
///     - P3: Exceeding max attempts triggers lockout - server returns
///     RequiredTimeDelayNotExpired until lockout expires.
///     - P4: Lockout expires after the configured duration
pub mod security;

/// DST property tests for DiagnosticSessionControl (0x10).
///
/// Properties tested:
///     - P1: After a valid DSC request, the server is in the requested session and client received
///     a positive response.
///     - P2 After DSC to extended, server returns to default if S3 expires.
///     - P3: DSC to an unsupported session returns NRC SubFunctionNotSupported.
///     - P4: Properties P1-P3 hold under light and chaos fault injection.
pub mod session;

/// DST property tests for RequestDownload (0x34) / TransferData (0x36) / RequestTransferExit (0x37).
///
/// The TestHandler returns ServiceNotSupported for these by default. These tests verify the server
/// correctly returns NRC 0x11 until the handler is extended to support them
///
/// Full flash programming tests will be added when ace-server gains a dedicated flash programming
/// hook model
pub mod transfer;

pub mod doip;

// region: Tick Parameters

pub const TICK_MS: Duration = Duration::from_millis(1);
pub const MAX_TICKS: usize = 500;

// endregion: Tick Parameters
