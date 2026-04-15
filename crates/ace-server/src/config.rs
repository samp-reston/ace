// region: Imports

use ace_sim::clock::Duration;

// endregion: Imports

// region: Periodic Rate Presets

/// Periodic transmission rate presets.
/// Arbitrary intervals are supported - these are provided for convenience.
pub mod periodic {
    use ace_sim::clock::Duration;

    /// 2000ms - Slow
    pub const SLOW: Duration = Duration::from_millis(2_000);

    /// 500ms - Medium
    pub const MEDIUM: Duration = Duration::from_millis(500);

    /// 50ms - Fast
    pub const FAST: Duration = Duration::from_millis(50);
}

// endregion: Periodic Rate Presets

// region: Session Config

/// Configuration for a single UDS diagnostic session.
///
/// Mirrors the session configuration in an ODX ECU-DESC container.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// UDS session type byte.
    /// 0x01 Default Session, 0x02 Programming Session, 0x03 ExtendedSession.
    pub session_type: u8,

    /// P2 server - max time to respond before the tester times out (ms)
    pub p2_timeout: Duration,

    /// P2* server - max time after sending 0x78 Response Pending before the final response must be
    /// sent (ms)
    pub p2_extended_timeout: Duration,

    /// S3 server - max time between Tester Present messages before the server drops back to
    /// Default Session (ms)
    pub s3_timeout: Duration,
}

impl SessionConfig {
    pub const fn default_session() -> Self {
        Self {
            session_type: 0x01,
            p2_timeout: Duration::from_millis(50),
            p2_extended_timeout: Duration::from_millis(5_000),
            s3_timeout: Duration::from_millis(5_000),
        }
    }

    pub const fn programming_session() -> Self {
        Self {
            session_type: 0x02,
            p2_timeout: Duration::from_millis(50),
            p2_extended_timeout: Duration::from_millis(5_000),
            s3_timeout: Duration::from_millis(5_000),
        }
    }

    pub const fn extended_session() -> Self {
        Self {
            session_type: 0x03,
            p2_timeout: Duration::from_millis(50),
            p2_extended_timeout: Duration::from_millis(5_000),
            s3_timeout: Duration::from_millis(5_000),
        }
    }
}

// endregion: Session Config

// region: Service Config

/// Configuration for a supported UDS service.
///
/// Mirrors a DiagService entry in an ODX file.
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// UDS service ID byte (e.g 0x22 for ReadDataByIdentifier).
    pub service_id: u8,

    /// Session types in which this service is available. The server returns 0x7F NRC Service Not
    /// Supported In Active Session if a request arrives outside these sessions.
    pub supported_in: &'static [u8],

    /// Minimum security level required. 0 = no security required.
    pub security_level: u8,
}

impl ServiceConfig {
    pub const fn new(service_id: u8, supported_in: &'static [u8]) -> Self {
        ServiceConfig {
            service_id,
            supported_in,
            security_level: 0,
        }
    }

    pub const fn secured(service_id: u8, supported_in: &'static [u8], security_level: u8) -> Self {
        Self {
            service_id,
            supported_in,
            security_level,
        }
    }
}

// endregion: Service Config

// region: DID Config

/// Configuration for a single Data Identifier (DID).
///
/// Mirrors a DataObject entry in an ODX file.
#[derive(Debug, Clone)]
pub struct DidConfig {
    /// 2-byte DID value.
    pub identifier: u16,

    /// Sessions in which this DID may be read. Empty = not readable.
    pub readable_in: &'static [u8],

    /// Sessions in which this DID may be written. Empty = not writable.
    pub writable_in: &'static [u8],

    /// Minimum security level to access this DID. 0 = no security.
    pub security_level: u8,

    /// Whether this DID may be scheduled for periodic transmission (0x2A).
    pub periodic: bool,

    /// Minimum interval the server will honor for periodic scheduling. Client-requested intervals
    /// shorter than this are clamped up.
    pub min_periodic_interval: Duration,
}

impl DidConfig {
    pub const fn read_only(identifier: u16, readable_in: &'static [u8]) -> Self {
        Self {
            identifier,
            readable_in,
            writable_in: &[],
            security_level: 0,
            periodic: false,
            min_periodic_interval: Duration::from_millis(50),
        }
    }

    pub const fn read_write(
        identifier: u16,
        readable_in: &'static [u8],
        writable_in: &'static [u8],
    ) -> Self {
        Self {
            identifier,
            readable_in,
            writable_in,
            security_level: 0,
            periodic: false,
            min_periodic_interval: Duration::from_millis(50),
        }
    }

    pub const fn periodic(mut self, min_interval: Duration) -> Self {
        self.periodic = true;
        self.min_periodic_interval = min_interval;
        self
    }

    pub const fn secured(mut self, level: u8) -> Self {
        self.security_level = level;
        self
    }
}

// endregion: DID Config

// region: Security Level Config

/// Configuration for a single security access level.
///
/// Mirrors a Security entry in an ODX file.
#[derive(Debug, Clone)]
pub struct SecurityLevelConfig {
    /// Request Seed byte for this level (always odd: 0x01, 0x03, 0x05 ...).
    pub level: u8,

    /// Max failed key attempts before lockout is applied.
    pub max_attempts: u8,

    /// Duration of the lockout after exceeding max attempts.
    pub lockout_duration: Duration,

    /// Expected seed length in bytes.
    pub seed_length: usize,

    /// Expected key length in bytes.
    pub key_length: usize,
}

// endregion: Security Level Config

// region: Server Config

/// Complete server configuration - mirrors what an ODX ECU description provides.
///
/// Constructed once (typically as a static or const) and referenced by the server state machine.
/// All lookups are O(n) over the small, fixed-size `heapless::Vec` collections - appropriate for
/// the sizes involved.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Physical address this server responds to.
    pub physical_address: u16,

    /// Functional (broadcast) address this server listens on.
    pub functional_address: u16,

    pub sessions: heapless::Vec<SessionConfig, 8>,
    pub services: heapless::Vec<ServiceConfig, 32>,
    pub data_identifiers: heapless::Vec<DidConfig, 64>,
    pub security_levels: heapless::Vec<SecurityLevelConfig, 8>,
}

impl ServerConfig {
    pub fn new(physical_address: u16, functional_address: u16) -> Self {
        Self {
            physical_address,
            functional_address,
            sessions: heapless::Vec::new(),
            services: heapless::Vec::new(),
            data_identifiers: heapless::Vec::new(),
            security_levels: heapless::Vec::new(),
        }
    }

    // region: Builder methods

    pub fn with_session(mut self, s: SessionConfig) -> Self {
        let _ = self.sessions.push(s);
        self
    }

    pub fn with_service(mut self, s: ServiceConfig) -> Self {
        let _ = self.services.push(s);
        self
    }

    pub fn with_did(mut self, d: DidConfig) -> Self {
        let _ = self.data_identifiers.push(d);
        self
    }
    pub fn with_security_level(mut self, l: SecurityLevelConfig) -> Self {
        let _ = self.security_levels.push(l);
        self
    }

    // endregion: Builder methods

    // region: Lookup helpers

    pub fn find_session(&self, session_type: u8) -> Option<&SessionConfig> {
        self.sessions
            .iter()
            .find(|s| s.session_type == session_type)
    }

    pub fn find_service(&self, service_id: u8) -> Option<&ServiceConfig> {
        self.services.iter().find(|s| s.service_id == service_id)
    }

    pub fn find_did(&self, identifier: u16) -> Option<&DidConfig> {
        self.data_identifiers
            .iter()
            .find(|d| d.identifier == identifier)
    }

    pub fn find_security_level(&self, level: u8) -> Option<&SecurityLevelConfig> {
        self.security_levels.iter().find(|l| l.level == level)
    }

    pub fn service_allowed(&self, service_id: u8, session_type: u8) -> bool {
        self.find_service(service_id)
            .map(|s| s.supported_in.contains(&session_type))
            .unwrap_or(false)
    }

    pub fn did_readable(&self, identifier: u16, session_type: u8) -> bool {
        self.find_did(identifier)
            .map(|s| s.readable_in.contains(&session_type))
            .unwrap_or(false)
    }

    pub fn did_writable(&self, identifier: u16, session_type: u8) -> bool {
        self.find_did(identifier)
            .map(|s| s.writable_in.contains(&session_type))
            .unwrap_or(false)
    }

    // endregion: Lookup helpers
}

// endregion: Server Config
