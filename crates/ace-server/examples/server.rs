use ace_core::Vec;
use ace_core::clock::Instant;
use ace_core::io::{InboundFrame, NodeAddress};
use ace_server::BuiltinNrc;
use ace_server::config::{DidConfig, ServerConfig, ServiceConfig, SessionConfig};
use ace_server::handler::ServerHandler;
use ace_server::security_provider::{SecurityError, SecurityProvider};
use ace_server::server::UdsServer;

const MAX_FRAME: usize = 32;
const MAX_OUTBOX: usize = 4;
const MAX_SESSIONS: usize = 4;
const MAX_SERVICES: usize = 8;
const MAX_DIDS: usize = 8;
const MAX_SECURITY_LEVELS: usize = 2;
const DEFAULT_S3: u64 = 5_000;
const DEFAULT_P2: u64 = 50;
const DEFAULT_P2_EXT: u64 = 5_000;
const DEFAULT_LOCKOUT: u64 = 10_000;
const MAX_ATTEMPTS: u8 = 3;
const MAX_SEED: usize = 4;
const MAX_PERIODIC: usize = 4;

type Server = UdsServer<
    MAX_FRAME,
    MAX_OUTBOX,
    MAX_SESSIONS,
    MAX_SERVICES,
    MAX_DIDS,
    MAX_SECURITY_LEVELS,
    DEFAULT_S3,
    DEFAULT_P2,
    DEFAULT_P2_EXT,
    DEFAULT_LOCKOUT,
    MAX_ATTEMPTS,
    MAX_SEED,
    MAX_PERIODIC,
    Handler,
    NoSecurity,
>;

struct Handler;

impl ServerHandler for Handler {
    type Error = BuiltinNrc;

    fn read_did(&self, did: u16, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if did != 0xF190 || buf.len() < 4 {
            return Err(BuiltinNrc::RequestOutOfRange);
        }
        buf[..4].copy_from_slice(b"ACE!");
        Ok(4)
    }

    fn write_did(&mut self, _did: u16, _data: &[u8]) -> Result<(), Self::Error> {
        Err(BuiltinNrc::RequestOutOfRange)
    }

    fn ecu_reset(&mut self, _reset_type: u8) -> Result<(), Self::Error> {
        Ok(())
    }
}

struct NoSecurity;

impl SecurityProvider for NoSecurity {
    fn generate_seed(&mut self, _level: u8, _buf: &mut [u8]) -> Result<usize, SecurityError> {
        Err(SecurityError::InvalidKey)
    }

    fn validate_key(&self, _level: u8, _seed: &[u8], _key: &[u8]) -> Result<(), SecurityError> {
        Err(SecurityError::InvalidKey)
    }
}

fn main() {
    let config: ServerConfig<MAX_SESSIONS, MAX_SERVICES, MAX_DIDS, MAX_SECURITY_LEVELS> =
        ServerConfig::new(0x0001, 0x7DF)
            .with_session(SessionConfig::default_session())
            .with_session(SessionConfig::extended_session())
            .with_service(ServiceConfig::new(0x22, &[0x01, 0x03]))
            .with_did(DidConfig::read_only(0xF190, &[0x01, 0x03]));

    let mut server: Server = UdsServer::new(config, Handler, NoSecurity, NodeAddress(0x0001));

    let request = InboundFrame::new(
        NodeAddress(0x0E00),
        NodeAddress(0x0001),
        &[0x22, 0xF1, 0x90],
    );
    server
        .handle(&request, Instant::ZERO)
        .expect("request handled");

    let mut outbox: Vec<(NodeAddress, Vec<u8, MAX_FRAME>), MAX_OUTBOX> = Vec::new();
    server.drain_outbox(&mut outbox).expect("outbox capacity");
    assert_eq!(
        outbox[0].1.as_slice(),
        &[0x62, 0xF1, 0x90, b'A', b'C', b'E', b'!']
    );
}
