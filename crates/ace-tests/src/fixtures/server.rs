// region: Imports

use ace_server::{
    config::{DidConfig, SecurityLevelConfig, ServerConfig, ServiceConfig, SessionConfig},
    handler::ServerHandler,
    security_provider::{SecurityError, SecurityProvider},
    server::UdsServer,
    BuiltinNrc,
};
use ace_sim::{clock::Duration, io::NodeAddress};
use heapless::Vec;

// endregion: Imports

// region: TestHandler

/// Minimal in-memory ServerHandler for DST tests.
///
/// Stores up to 8 DIDs with up to 64 bytes of data each. All optional hooks return
/// serviceNotSupported by default.
pub struct TestHandler {
    dids: Vec<(u16, Vec<u8, 64>), 8>,
}

impl TestHandler {
    pub fn new() -> Self {
        Self { dids: Vec::new() }
    }

    pub fn set_did(&mut self, id: u16, data: &[u8]) {
        if let Some(entry) = self.dids.iter_mut().find(|(d, _)| *d == id) {
            entry.1.clear();
            let _ = entry.1.extend_from_slice(&data[..data.len().min(64)]);
        } else {
            let mut buf: Vec<u8, 64> = Vec::new();
            let _ = buf.extend_from_slice(&data[..data.len().min(64)]);

            let _ = self.dids.push((id, buf));
        }
    }
}

impl Default for TestHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerHandler for TestHandler {
    type Error = BuiltinNrc;

    fn read_did(&self, did: u16, buf: &mut [u8]) -> Result<usize, Self::Error> {
        match self.dids.iter().find(|(d, _)| *d == did) {
            Some((_, data)) => {
                let len = data.len().min(buf.len());
                buf[..len].copy_from_slice(&data[..len]);

                Ok(len)
            }
            None => Err(BuiltinNrc::RequestOutOfRange),
        }
    }

    fn write_did(&mut self, did: u16, data: &[u8]) -> Result<(), Self::Error> {
        if let Some(entry) = self.dids.iter_mut().find(|(d, _)| *d == did) {
            entry.1.clear();
            let _ = entry.1.extend_from_slice(&data[..data.len().min(64)]);

            Ok(())
        } else {
            Err(BuiltinNrc::RequestOutOfRange)
        }
    }

    fn ecu_reset(&mut self, _reset_type: u8) -> Result<(), Self::Error> {
        Ok(())
    }
}

// endregion: TestHandler

// region: TestSecurityProvider

/// Fixed XOR security provider for DST tests.
///
/// Seed is the level byte. Key = seed XOR 0xFF. Deterministic and trivially predictable - suitable
/// only for testing.
pub struct TestSecurityProvider;

impl SecurityProvider for TestSecurityProvider {
    fn generate_seed(
        &mut self,
        level: u8,
        buf: &mut [u8],
    ) -> Result<usize, ace_server::security_provider::SecurityError> {
        if buf.is_empty() {
            return Err(SecurityError::InvalidKey);
        }

        buf[0] = level;
        Ok(1)
    }

    fn validate_key(&self, _level: u8, seed: &[u8], key: &[u8]) -> Result<(), SecurityError> {
        let expected_key = seed.first().copied().unwrap_or(0) ^ 0xFF;

        match key.first().copied() {
            Some(k) if k == expected_key => Ok(()),
            _ => Err(SecurityError::InvalidKey),
        }
    }
}

// endregion: TestSecurityProvider

// region: Default Server Factory

/// Builds a default test server with a realistic ODX-style configuration.
///
/// Sessions: default, programming, extended
/// Services: DSC, EcuReset, SecurityAccess, RDBI, WEDBI, RC, TesterPresent
/// Dids:
///     - 0xF190 (VIN, read-only, all sessions)
///     - 0xF101 (ECU serial, read-only, extended + programming)
///     - 0xF120 (application version, read/write, extended, security level 1)
/// Security: level 1, max 3 attempts, 10s lockout, 1 byte seed/key
pub fn default_server(address: NodeAddress) -> UdsServer<TestHandler, TestSecurityProvider> {
    let mut handler = TestHandler::new();
    handler.set_did(0xF190, b"TESTVIN1234567890");
    handler.set_did(0xF290, b"TESTVIN1234567890");
    handler.set_did(0xF101, b"SN-001");
    handler.set_did(0xF120, &[0x01, 0x02]);

    let config = ServerConfig::new(address.0 as u16, 0x7DF)
        .with_session(SessionConfig::default_session())
        .with_session(SessionConfig::programming_session())
        .with_session(SessionConfig::extended_session())
        .with_service(ServiceConfig::new(0x10, &[0x01, 0x02, 0x03]))
        .with_service(ServiceConfig::new(0x11, &[0x01, 0x02, 0x03]))
        .with_service(ServiceConfig::new(0x27, &[0x01, 0x02, 0x03]))
        .with_service(ServiceConfig::new(0x22, &[0x01, 0x02, 0x03]))
        .with_service(ServiceConfig::new(0x2E, &[0x03]))
        .with_service(ServiceConfig::new(0x2A, &[0x01, 0x02, 0x03]))
        .with_service(ServiceConfig::new(0x31, &[0x02, 0x03]))
        .with_service(ServiceConfig::new(0x3E, &[0x01, 0x02, 0x03]))
        .with_service(ServiceConfig::new(0x34, &[0x02]))
        .with_did(DidConfig::read_only(0xF190, &[0x01, 0x02, 0x03]))
        .with_did(DidConfig::read_only(0xF290, &[0x01, 0x02, 0x03]))
        .with_did(DidConfig::read_only(0xF101, &[0x02, 0x03]))
        .with_did(DidConfig::read_write(0xF120, &[0x02, 0x03], &[0x03]).secured(1))
        .with_security_level(SecurityLevelConfig {
            level: 0x01,
            max_attempts: 3,
            lockout_duration: Duration::from_millis(10_000),
            seed_length: 1,
            key_length: 1,
        });

    UdsServer::new(config, handler, TestSecurityProvider, address)
}

// endregion: Default Server Factory
