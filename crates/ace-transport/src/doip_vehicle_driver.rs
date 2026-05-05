//! Production wiring - connects DoipTester to a real vehicle using std::net::TcpStream for
//! DoIP/TCP and std::net::UdpSocket for DoIP/UDP vehicle discovery.
//!
//! The DoipTester state machine is completely unchanged - it produces and consumes raw bytes. This
//! driver is the transport boundary:
//!
//! TcpStream -> read raw bytes -> DoipTester::handle()
//! DoipTester outbox -> write raw bytes -> TcpStream
//!
//! UdpSocket -> read announcement -> DoipTester::handle()
//! DoipTester outbox -> write discovery request -> UdpSocket
//!
//! Threading model:
//!     - One reader thread per TCP connection - blocks on TcpStream::read
//!     - One writer thread per TCP connection - drains outbox to TcpStream
//!     - One UDP thread - handles announcements and discovery responses
//!     - Main thread drives DoipTester::tick() on a configurable interval
//!
//! All communication between threads goes through std::sync channels.

// region: Imports

use ace_doip::header::ProtocolVersion;
use ace_gateway::tester::{
    ConnectionId, DoipConnectionConfig, DoipConnectionPhase, DoipNodeProfile, DoipTester,
    DoipTesterError, DoipTesterEvent, TargetId,
};
use ace_sim::io::NodeAddress;
use core::time::Duration;
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream, UdpSocket},
    sync::mpsc::{self, channel, Receiver, Sender},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

// endregion: Imports

// region: InboundMessage

/// Raw bytes received from the network, tagged with their source connection.
struct InboundMessage {
    connection_id: ConnectionId,
    data: Vec<u8>,
}

// endregion: InboundMessage

// region: VehicleDriverConfig

/// Configuration for the production DoIP vehicle driver.
pub struct VehicleDriverConfig {
    /// Logical address of this tester device.
    pub tester_address: u16,

    /// How often to call DoipTester::tick() - drives P2 timeouts.
    pub tick_interval: Duration,

    /// Read buffer size per TCP connection.
    pub read_buf_size: usize,

    /// UDP port for DoIP discovery.
    pub udp_port: u16,

    /// UDP multicast address for vehicle announcement.
    pub udp_multicast_addr: &'static str,

    /// The default protocol version used.
    pub default_protocol_version: ProtocolVersion,
}

impl VehicleDriverConfig {
    pub fn new(
        tester_address: u16,
        tick_interval: Duration,
        read_buf_size: usize,
        udp_multicast_addr: &'static str,
        protocol: ProtocolVersion,
    ) -> Self {
        Self {
            tester_address,
            tick_interval,
            read_buf_size,
            udp_port: 13400,
            udp_multicast_addr,
            default_protocol_version: protocol,
        }
    }
}

// endregion: VehicleDriverConfig

// region: TcpConnectionHandle

/// Handle to a live TCP connection - used by the writer thread to send frames.
struct _TcpConnectionHandle {
    connection_id: ConnectionId,
    stream: TcpStream,
}

// endregion: TcpConnectionHandle

// region: DoipVehicleDriver

/// Production DoIP tester driver.
///
/// Wraps `DoipTester` with real TCP and UDP transport. All network I/O runs on background threads.
/// The application calls `tick()` in a loop or on a timer to drive the state machine and drain
/// events.
///
/// # Usage
///
/// ```ignore
///
/// let mut driver = DoipVehicleDriver::new(VehicleDriverConfig::new(0x0E00,
/// Duration::from_millis(1), 65535, 13400, "0.0.0.0"));
///
/// // Connect to a gateway at a known IP
/// let conn_id = driver.connect(
///     "192.168.1.10:13400".parse().unwrap(),
///     DoipConnectionConfig::new(0x0E00),
/// )?;
///
/// // Main Loop
/// loop {
///     driver.tick();
///
///     for (conn, target, event) in driver.drain_events() {
///         match event {
///             DoipTesterEvent::ActivationSucceeded => {
///                 driver.request(conn, 0x0001, &[0x10, 0x03]).unwrap();
///             }
///             DoipTesterEvent::Uds(ClientEvent::PositiveResponse { sid, data}) => {
///                 println!("Response for SID 0x{:02X}: {:02X?}", sid, data.as_slice());
///             }
///             _ => {}
///         }
///     }
///
///     std::thread::sleep(driver.config.tick_interval);
/// }
///
/// ```
pub struct DoipVehicleDriver {
    pub config: VehicleDriverConfig,
    tester: DoipTester<8, 16>,

    /// Inbound bytes from TCP reader threads.
    tcp_rx: Receiver<InboundMessage>,
    tcp_tx_clone: Sender<InboundMessage>,

    /// Outbound bytes for each TCP connection.
    /// Each entry is (ConnectionId, Sender to that connection's writer thread).
    tcp_writers: Vec<(ConnectionId, Sender<Vec<u8>>)>,

    /// Inbound bytes from the UDP reader thread.
    udp_rx: Receiver<Vec<u8>>,
}

impl DoipVehicleDriver {
    pub fn new(config: VehicleDriverConfig) -> Self {
        let (tcp_tx, tcp_rx) = mpsc::channel::<InboundMessage>();
        let (udp_tx, udp_rx) = mpsc::channel::<Vec<u8>>();

        let tester_addr = NodeAddress(config.tester_address as u32);
        let tester = DoipTester::new(config.tester_address, tester_addr);

        {
            let udp_port = config.udp_port;
            std::thread::spawn(move || {
                udp_listener_thread(udp_tx, udp_port);
            });
        }

        Self {
            config,
            tester,
            tcp_rx,
            tcp_tx_clone: tcp_tx,
            tcp_writers: Vec::new(),
            udp_rx,
        }
    }

    // region: Connection Management

    /// Opens a TCP connection to a DoIP gateway at `addr`.
    ///
    /// Spawns a reader thread and a write thread for this connection. The `DoipTester` state
    /// machine will send a `RoutingActivationRequest` automatically once the TCP connection is
    /// estabished.
    pub fn connect(
        &mut self,
        addr: SocketAddr,
        config: DoipConnectionConfig,
    ) -> std::io::Result<ConnectionId> {
        let conn_id = ConnectionId(config.gateway_address);

        let stream = TcpStream::connect(addr)?;
        stream.set_nodelay(true)?;

        let write_stream = stream.try_clone()?;
        let read_stream = stream;

        let (write_tx, write_rx) = channel::<Vec<u8>>();

        {
            let inbound_tx = self.tcp_tx_clone.clone();
            let buf_size = self.config.read_buf_size;
            std::thread::spawn(move || {
                tcp_reader_thread(read_stream, conn_id, inbound_tx, buf_size);
            });
        }

        std::thread::spawn(move || {
            tcp_writer_thread(write_stream, write_rx);
        });

        self.tcp_writers.push((conn_id, write_tx));

        self.tester.open_connection(config).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "DoipTester: too many connections",
            )
        })?;

        let now = ace_sim::clock::Instant::from_micros(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
        );

        if let Some(conn) = self.tester.find_conn_mut(conn_id) {
            let _ = conn.on_connected(now);
        }

        Ok(conn_id)
    }

    /// Disconnects from a gateway cleanly.
    pub fn disconnect(&mut self, conn_id: ConnectionId) {
        self.tcp_writers.retain(|(id, _)| *id != conn_id);
    }

    // endregion: Connection Management

    // region: Main loop

    /// Drives the tester state machine - call this on a regular interval.
    ///
    /// Reads all pending inbound bytes from TCP and UDP threads, delivers them to the tester,
    /// ticks the tester timers, and flushes the outbox to the appropriate TCP connections.
    pub fn tick(&mut self) {
        let now = self.wall_clock_now();

        while let Ok(msg) = self.tcp_rx.try_recv() {
            let _ = self
                .tester
                .handle(&NodeAddress(msg.connection_id.0 as u32), &msg.data, now);
        }

        while let Ok(data) = self.udp_rx.try_recv() {
            for (conn_id, _) in &self.tcp_writers {
                let gateway_addr = NodeAddress(conn_id.0 as u32);
                let _ = self.tester.handle(&gateway_addr, &data, now);
            }
        }

        self.tester.tick(now);

        self.flush_outbox();
    }

    // endregion: Main loop

    // region: Request / Event API

    /// Sends a UDS request to a target ECU over a specific connection.
    pub fn request(
        &mut self,
        conn_id: ConnectionId,
        target_address: u16,
        uds_data: &[u8],
    ) -> Result<(), DoipTesterError> {
        let now = self.wall_clock_now();
        self.tester.request(conn_id, target_address, uds_data, now)
    }

    /// Subscribes to periodic DID data from a specific ECU.
    pub fn subscribe_periodic(
        &mut self,
        conn_id: ConnectionId,
        target_address: u16,
        did_low_byte: u8,
    ) {
        self.tester
            .subscribe_periodic(conn_id, target_address, did_low_byte);
    }

    /// Drains all accumulated tester events.
    pub fn drain_events(
        &mut self,
    ) -> impl Iterator<Item = (ConnectionId, TargetId, DoipTesterEvent)> + '_ {
        self.tester.drain_events()
    }

    pub fn connection_phase(&self, conn_id: ConnectionId) -> Option<&DoipConnectionPhase> {
        self.tester.connection_phase(conn_id)
    }

    /// Returns the node profile for a gateway, populated from `Vehicle AnnouncementMessage` and
    /// `EntityStatusResponse` frames.
    pub fn node_profile(&self, gateway_address: u16) -> Option<&DoipNodeProfile> {
        self.tester.profile(gateway_address)
    }

    // endregion: Request / Event API

    // region: Internal Helpers

    fn flush_outbox(&mut self) {
        let mut outbox: heapless::Vec<
            (
                NodeAddress,
                heapless::Vec<u8, { ace_gateway::gateway::TCP_MAX_FRAME }>,
            ),
            { ace_gateway::gateway::TCP_MAX_OUTBOX },
        > = heapless::Vec::new();

        self.tester.drain_outbox(&mut outbox);

        for (dst, data) in &outbox {
            let conn_id = ConnectionId(dst.0 as u16);

            if let Some((_, writer)) = self.tcp_writers.iter().find(|(id, _)| *id == conn_id) {
                let _ = writer.send(data.to_vec());
            }
        }
    }

    fn wall_clock_now(&self) -> ace_sim::clock::Instant {
        ace_sim::clock::Instant::from_micros(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
        )
    }

    // endregion: Internal Helpers
}

// endregion: DoipVehicleDriver

// region: Background Threads

/// Reads raw bytes from a TCP stream and forwards them to the driver via the inbound channel. One
/// thread per connection.
///
/// DoIP frames may span multiple TCP reads - this reader accumulates bytes until a complete DoIP
/// frame is available before forwarding. A complete frame is identified by reading the 8-byte
/// header first, then reading exactly `payload_length` more bytes.
fn tcp_reader_thread(
    mut stream: TcpStream,
    connection_id: ConnectionId,
    tx: Sender<InboundMessage>,
    buf_size: usize,
) {
    const DOIP_HEADER_LEN: usize = 8;
    let mut header_buf = [0u8; DOIP_HEADER_LEN];

    loop {
        if stream.read_exact(&mut header_buf).is_err() {
            break;
        }

        let payload_len =
            u32::from_be_bytes([header_buf[4], header_buf[5], header_buf[6], header_buf[7]])
                as usize;

        if payload_len > buf_size {
            break;
        }

        let mut frame = Vec::with_capacity(DOIP_HEADER_LEN + payload_len);
        frame.extend_from_slice(&header_buf);

        if payload_len > 0 {
            frame.resize(DOIP_HEADER_LEN + payload_len, 0);
            if stream.read_exact(&mut frame[DOIP_HEADER_LEN..]).is_err() {
                break;
            }
        }

        if tx
            .send(InboundMessage {
                connection_id,
                data: frame,
            })
            .is_err()
        {
            break;
        }
    }
}

/// Writes outbound frames to a TCP stream. One thread per connection.
///
/// Blocks waiting for frames from the channel. Exits when the channel is closed (driver dropped or
/// explicit disconnect).
fn tcp_writer_thread(mut stream: TcpStream, rx: Receiver<Vec<u8>>) {
    while let Ok(frame) = rx.recv() {
        if stream.write_all(&frame).is_err() {
            break;
        }
    }
}

/// Listens on the DoIP UDP port for vehicle announcements and entity status responses. Forwards
/// raw frames to the driver.
///
/// Binds to `0.0.0.0:{udp_port}` and reads datagrams. DoIP UDP frames are complete in a single
/// datagram so no reassembly needed.
fn udp_listener_thread(tx: Sender<Vec<u8>>, udp_port: u16) {
    let bind_addr = format!("0.0.0.0:{}", udp_port);
    let socket = match UdpSocket::bind(&bind_addr) {
        Ok(s) => s,
        Err(_) => return,
    };

    let mut buf = vec![0u8; 4096];
    loop {
        match socket.recv_from(&mut buf) {
            Ok((len, _src)) => {
                let frame = buf[..len].to_vec();
                if tx.send(frame).is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

// endregion: Background Threads

// region: DiscoveryConfig

/// Configuration for DoIP UDP gateway discovery.
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// DoIP UDP port - ISO 13400-2 default is 13400.
    pub udp_port: u16,

    /// UDP multicast address for vehicle announcements.
    pub multicast_addr: String,

    /// How long to wait for gateway responses after sending the request.
    pub timeout: Duration,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            udp_port: 13400,
            multicast_addr: "0.0.0.0".to_string(),
            timeout: Duration::from_secs(2),
        }
    }
}

// endregion: DiscoveryConfig

// region: UDP Discovery

/// Sends a `VehicleIdentificationRequest` to the DoIP multicast group and returns the addresses of
/// gateways that responded within the timeout.
///
/// The discovery frame's protocol version is taken from `config.protocol_version` - set this to
/// match the protocol version expected by the target vehicles. The payload type `0x0001`
/// (VehicleIdentificationRequest) and payload length `0` are fixed and are not configurable.
///
/// Callers use the returned addresses with `DoipVehicleDriver::connect()`.
pub fn discover_gateways(
    config: &DiscoveryConfig,
    protocol: ProtocolVersion,
) -> std::io::Result<Vec<SocketAddr>> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(config.timeout))?;

    let version = protocol as u8;
    let inverse = !version;
    let frame = [version, inverse, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00];

    let target = format!("{}:{}", config.multicast_addr, config.udp_port);
    socket.send_to(&frame, &target)?;

    let mut discovered = Vec::new();
    let mut buf = [0u8; 4096];
    let deadline = Instant::now() + config.timeout;

    loop {
        if Instant::now() >= deadline {
            break;
        }

        match socket.recv_from(&mut buf) {
            Ok((_, src)) => {
                if !discovered.contains(&src) {
                    discovered.push(src);
                }
            }
            Err(_) => break,
        }
    }

    Ok(discovered)
}

// endregion: UDP Discovery
