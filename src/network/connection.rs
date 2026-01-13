//! Connection management for P2P audio streaming

use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use tokio::time::interval;
use tracing::{debug, info, trace, warn};

use crate::protocol::{LatencyInfoMessage, LatencyPing, LatencyPong, Packet, PacketType};

use super::error::NetworkError;
use super::transport::UdpTransport;

/// Number of RTT samples to keep for averaging
const RTT_SAMPLE_COUNT: usize = 10;

/// Maximum pending pings before discarding old ones
const MAX_PENDING_PINGS: usize = 10;

/// Connection statistics
#[derive(Debug, Clone, Default)]
pub struct ConnectionStats {
    /// Round-trip time in milliseconds
    pub rtt_ms: f32,
    /// Packet loss rate (0.0 - 1.0)
    pub packet_loss_rate: f32,
    /// Jitter in milliseconds (RTT variation)
    pub jitter_ms: f32,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Total packets sent
    pub packets_sent: u64,
    /// Total packets received
    pub packets_received: u64,
    /// Connection uptime in seconds
    pub uptime_seconds: u64,
}

/// RTT measurement state
#[derive(Debug)]
struct RttMeasurement {
    /// Current smoothed RTT estimate (ms)
    rtt_ms: f32,
    /// RTT jitter / variation (ms)
    jitter_ms: f32,
    /// Pending ping sequences with sent timestamps (monotonic instant)
    pending_pings: HashMap<u32, Instant>,
    /// Recent RTT samples for averaging
    rtt_samples: VecDeque<f32>,
    /// Next ping sequence number
    next_ping_seq: u32,
    /// Monotonic time reference for timestamps
    time_reference: Instant,
}

impl Default for RttMeasurement {
    fn default() -> Self {
        Self {
            rtt_ms: 0.0,
            jitter_ms: 0.0,
            pending_pings: HashMap::new(),
            rtt_samples: VecDeque::with_capacity(RTT_SAMPLE_COUNT),
            next_ping_seq: 0,
            time_reference: Instant::now(),
        }
    }
}

impl RttMeasurement {
    /// Get current time in microseconds since reference
    fn now_us(&self) -> u64 {
        self.time_reference.elapsed().as_micros() as u64
    }

    /// Create a ping message and record the send time
    fn create_ping(&mut self) -> LatencyPing {
        let seq = self.next_ping_seq;
        self.next_ping_seq = self.next_ping_seq.wrapping_add(1);

        // Clean up old pending pings if too many
        if self.pending_pings.len() >= MAX_PENDING_PINGS {
            // Remove the oldest entry
            if let Some(&oldest_seq) = self.pending_pings.keys().min() {
                self.pending_pings.remove(&oldest_seq);
            }
        }

        self.pending_pings.insert(seq, Instant::now());

        LatencyPing {
            sent_time_us: self.now_us(),
            ping_sequence: seq,
        }
    }

    /// Process a pong response and update RTT statistics
    fn process_pong(&mut self, pong: &LatencyPong) {
        if let Some(sent_time) = self.pending_pings.remove(&pong.ping_sequence) {
            let rtt = sent_time.elapsed().as_secs_f32() * 1000.0; // Convert to ms

            // Update RTT samples
            if self.rtt_samples.len() >= RTT_SAMPLE_COUNT {
                self.rtt_samples.pop_front();
            }
            self.rtt_samples.push_back(rtt);

            // Calculate smoothed RTT (exponential moving average)
            let alpha = 0.125; // Standard TCP-like smoothing factor
            if self.rtt_ms == 0.0 {
                self.rtt_ms = rtt;
            } else {
                self.rtt_ms = (1.0 - alpha) * self.rtt_ms + alpha * rtt;
            }

            // Calculate jitter (RTT variation)
            let beta = 0.25;
            let diff = (rtt - self.rtt_ms).abs();
            self.jitter_ms = (1.0 - beta) * self.jitter_ms + beta * diff;

            trace!(
                "RTT updated: rtt={:.2}ms, jitter={:.2}ms (sample={:.2}ms)",
                self.rtt_ms,
                self.jitter_ms,
                rtt
            );
        }
    }
}

/// Peer latency information received from remote peer
#[derive(Debug, Clone, Default)]
pub struct PeerLatencyInfo {
    /// Peer's capture buffer latency (ms)
    pub capture_buffer_ms: f32,
    /// Peer's playback buffer latency (ms)
    pub playback_buffer_ms: f32,
    /// Peer's encode latency (ms)
    pub encode_ms: f32,
    /// Peer's decode latency (ms)
    pub decode_ms: f32,
    /// Peer's current jitter buffer delay (ms)
    pub jitter_buffer_ms: f32,
    /// Peer's frame size (samples)
    pub frame_size: u32,
    /// Peer's sample rate (Hz)
    pub sample_rate: u32,
    /// Peer's codec name
    pub codec: String,
}

impl From<LatencyInfoMessage> for PeerLatencyInfo {
    fn from(msg: LatencyInfoMessage) -> Self {
        Self {
            capture_buffer_ms: msg.capture_buffer_ms,
            playback_buffer_ms: msg.playback_buffer_ms,
            encode_ms: msg.encode_ms,
            decode_ms: msg.decode_ms,
            jitter_buffer_ms: msg.jitter_buffer_ms,
            frame_size: msg.frame_size,
            sample_rate: msg.sample_rate,
            codec: msg.codec,
        }
    }
}

/// Connection state
///
/// State machine for P2P connections with ICE/NAT traversal support.
///
/// ```text
/// [*] --> Disconnected
/// Disconnected --> Connecting: connect()
/// Connecting --> GatheringCandidates: ICE start
/// GatheringCandidates --> CheckingConnectivity: candidates ready
/// CheckingConnectivity --> Connected: ICE success
/// CheckingConnectivity --> Failed: ICE failed
/// Connected --> Reconnecting: connection lost
/// Reconnecting --> Connected: reconnect success
/// Reconnecting --> Failed: timeout
/// Connected --> Disconnected: disconnect()
/// Failed --> Disconnected: reset()
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ConnectionState {
    /// Not connected
    #[default]
    Disconnected = 0,
    /// Initiating connection
    Connecting = 1,
    /// Gathering ICE candidates
    GatheringCandidates = 2,
    /// Checking connectivity with candidates
    CheckingConnectivity = 3,
    /// Successfully connected
    Connected = 4,
    /// Attempting to reconnect after connection loss
    Reconnecting = 5,
    /// Connection failed
    Failed = 6,
}

impl ConnectionState {
    /// Convert from u8 value
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Disconnected,
            1 => Self::Connecting,
            2 => Self::GatheringCandidates,
            3 => Self::CheckingConnectivity,
            4 => Self::Connected,
            5 => Self::Reconnecting,
            6 => Self::Failed,
            _ => Self::Disconnected,
        }
    }

    /// Check if the connection is in a connected state
    pub fn is_connected(&self) -> bool {
        matches!(self, Self::Connected)
    }

    /// Check if the connection is in progress
    pub fn is_connecting(&self) -> bool {
        matches!(
            self,
            Self::Connecting | Self::GatheringCandidates | Self::CheckingConnectivity
        )
    }

    /// Check if the connection can send/receive data
    pub fn can_transmit(&self) -> bool {
        matches!(self, Self::Connected | Self::Reconnecting)
    }
}

/// Callback for received audio data
pub type AudioCallback = Box<dyn Fn(&[u8], u32) + Send + Sync + 'static>;

/// Callback for received peer latency info
pub type LatencyInfoCallback = Box<dyn Fn(PeerLatencyInfo) + Send + Sync + 'static>;

/// A P2P connection to a remote peer
pub struct Connection {
    transport: Arc<UdpTransport>,
    remote_addr: SocketAddr,
    state: Arc<AtomicU8>,
    /// Last error message that caused connection failure (if any)
    last_error: Arc<std::sync::Mutex<Option<String>>>,
    sequence: AtomicU32,
    packets_sent: Arc<AtomicU64>,
    packets_received: Arc<AtomicU64>,
    bytes_sent: Arc<AtomicU64>,
    bytes_received: Arc<AtomicU64>,
    last_received: Arc<std::sync::Mutex<Instant>>,
    audio_callback: Option<Arc<AudioCallback>>,
    receive_handle: Option<tokio::task::JoinHandle<()>>,
    keepalive_handle: Option<tokio::task::JoinHandle<()>>,
    /// RTT measurement state
    rtt_measurement: Arc<RwLock<RttMeasurement>>,
    /// Peer latency information (received from remote peer)
    peer_latency_info: Arc<RwLock<Option<PeerLatencyInfo>>>,
    /// Callback for when peer latency info is received
    latency_info_callback: Option<Arc<LatencyInfoCallback>>,
    /// Connection start time for uptime tracking
    connection_start: Arc<std::sync::Mutex<Option<Instant>>>,
}

impl Connection {
    /// Create a new connection (not yet connected)
    pub async fn new(local_addr: &str) -> Result<Self, NetworkError> {
        let transport = UdpTransport::bind(local_addr).await?;

        Ok(Self {
            transport: Arc::new(transport),
            remote_addr: "0.0.0.0:0".parse().unwrap(),
            state: Arc::new(AtomicU8::new(ConnectionState::Disconnected as u8)),
            last_error: Arc::new(std::sync::Mutex::new(None)),
            sequence: AtomicU32::new(0),
            packets_sent: Arc::new(AtomicU64::new(0)),
            packets_received: Arc::new(AtomicU64::new(0)),
            bytes_sent: Arc::new(AtomicU64::new(0)),
            bytes_received: Arc::new(AtomicU64::new(0)),
            last_received: Arc::new(std::sync::Mutex::new(Instant::now())),
            audio_callback: None,
            receive_handle: None,
            keepalive_handle: None,
            rtt_measurement: Arc::new(RwLock::new(RttMeasurement::default())),
            peer_latency_info: Arc::new(RwLock::new(None)),
            latency_info_callback: None,
            connection_start: Arc::new(std::sync::Mutex::new(None)),
        })
    }

    /// Get the local address
    pub fn local_addr(&self) -> SocketAddr {
        self.transport.local_addr()
    }

    /// Connect to a remote peer
    pub async fn connect(&mut self, remote_addr: SocketAddr) -> Result<(), NetworkError> {
        if self.is_connected() {
            return Err(NetworkError::AlreadyConnected);
        }

        self.remote_addr = remote_addr;
        self.set_state(ConnectionState::Connecting);
        info!("Connecting to {}", remote_addr);

        // Send initial keep-alive to establish connection
        let packet = Packet::keep_alive(self.next_sequence());
        self.transport.send_to(&packet, remote_addr).await?;

        // Record connection start time
        if let Ok(mut start) = self.connection_start.lock() {
            *start = Some(Instant::now());
        }

        self.set_state(ConnectionState::Connected);
        self.start_receive_loop();
        self.start_keepalive_loop();

        info!("Connected to {}", remote_addr);
        Ok(())
    }

    /// Disconnect from the remote peer
    pub fn disconnect(&mut self) {
        self.set_state(ConnectionState::Disconnected);

        if let Some(handle) = self.receive_handle.take() {
            handle.abort();
        }
        if let Some(handle) = self.keepalive_handle.take() {
            handle.abort();
        }

        info!("Disconnected from {}", self.remote_addr);
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.state().is_connected()
    }

    /// Get current connection state
    pub fn state(&self) -> ConnectionState {
        ConnectionState::from_u8(self.state.load(Ordering::SeqCst))
    }

    /// Set connection state
    fn set_state(&self, state: ConnectionState) {
        // Clear last_error when transitioning to a non-failed state
        if state != ConnectionState::Failed {
            if let Ok(mut err) = self.last_error.lock() {
                *err = None;
            }
        }
        self.state.store(state as u8, Ordering::SeqCst);
    }

    /// Set connection to failed state with error information
    #[allow(dead_code)]
    fn set_failed(&self, error: &NetworkError) {
        if let Ok(mut err) = self.last_error.lock() {
            *err = Some(error.to_string());
        }
        self.state
            .store(ConnectionState::Failed as u8, Ordering::SeqCst);
    }

    /// Get the last error message that caused connection failure
    ///
    /// Returns `None` if the connection has not failed or if no error was recorded.
    pub fn last_error(&self) -> Option<String> {
        self.last_error.lock().ok().and_then(|e| e.clone())
    }

    /// Set callback for received audio data
    pub fn set_audio_callback<F>(&mut self, callback: F)
    where
        F: Fn(&[u8], u32) + Send + Sync + 'static,
    {
        self.audio_callback = Some(Arc::new(Box::new(callback)));
    }

    /// Set callback for received peer latency info
    pub fn set_latency_info_callback<F>(&mut self, callback: F)
    where
        F: Fn(PeerLatencyInfo) + Send + Sync + 'static,
    {
        self.latency_info_callback = Some(Arc::new(Box::new(callback)));
    }

    /// Send latency info to the remote peer
    pub async fn send_latency_info(&self, info: &LatencyInfoMessage) -> Result<(), NetworkError> {
        if !self.state().can_transmit() {
            return Err(NetworkError::NotConnected);
        }

        let packet = Packet::latency_info(self.next_sequence(), info);
        self.transport.send_to(&packet, self.remote_addr).await?;

        debug!("Sent latency info to {}", self.remote_addr);
        Ok(())
    }

    /// Get the current RTT estimate in milliseconds
    pub fn rtt_ms(&self) -> f32 {
        self.rtt_measurement.read().rtt_ms
    }

    /// Get the current jitter estimate in milliseconds
    pub fn jitter_ms(&self) -> f32 {
        self.rtt_measurement.read().jitter_ms
    }

    /// Get the peer's latency information (if received)
    pub fn peer_latency_info(&self) -> Option<PeerLatencyInfo> {
        self.peer_latency_info.read().clone()
    }

    /// Send audio data to the remote peer
    pub async fn send_audio(&self, data: &[f32], timestamp: u32) -> Result<(), NetworkError> {
        if !self.state().can_transmit() {
            return Err(NetworkError::NotConnected);
        }

        // Convert f32 samples to bytes (little-endian)
        let bytes: Vec<u8> = data.iter().flat_map(|&s| s.to_le_bytes()).collect();

        let packet = Packet::audio(self.next_sequence(), timestamp, bytes);
        let packet_bytes = packet.to_bytes();
        let len = packet_bytes.len() as u64;

        self.transport.send_to(&packet, self.remote_addr).await?;

        self.packets_sent.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent.fetch_add(len, Ordering::Relaxed);

        Ok(())
    }

    /// Get connection statistics
    pub fn stats(&self) -> ConnectionStats {
        let rtt = self.rtt_measurement.read();
        let uptime = self
            .connection_start
            .lock()
            .ok()
            .and_then(|start| start.map(|s| s.elapsed().as_secs()))
            .unwrap_or(0);

        ConnectionStats {
            rtt_ms: rtt.rtt_ms,
            packet_loss_rate: 0.0, // TODO: Implement packet loss tracking
            jitter_ms: rtt.jitter_ms,
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            packets_sent: self.packets_sent.load(Ordering::Relaxed),
            packets_received: self.packets_received.load(Ordering::Relaxed),
            uptime_seconds: uptime,
        }
    }

    fn next_sequence(&self) -> u32 {
        self.sequence.fetch_add(1, Ordering::Relaxed)
    }

    fn start_receive_loop(&mut self) {
        let transport = self.transport.clone();
        let state = self.state.clone();
        let last_received = self.last_received.clone();
        let packets_received = self.packets_received.clone();
        let bytes_received = self.bytes_received.clone();
        let audio_callback = self.audio_callback.clone();
        let rtt_measurement = self.rtt_measurement.clone();
        let peer_latency_info = self.peer_latency_info.clone();
        let latency_info_callback = self.latency_info_callback.clone();
        let remote_addr = self.remote_addr;
        let sequence = Arc::new(AtomicU32::new(1_000_000)); // Separate sequence for pong responses

        let handle = tokio::spawn(async move {
            let (mut rx, _recv_handle) = transport.clone().start_receive_loop();

            while let Some((packet, _addr)) = rx.recv().await {
                let current_state = ConnectionState::from_u8(state.load(Ordering::SeqCst));
                if !current_state.can_transmit() {
                    break;
                }

                *last_received.lock().unwrap() = Instant::now();
                packets_received.fetch_add(1, Ordering::Relaxed);
                bytes_received.fetch_add(packet.payload.len() as u64 + 12, Ordering::Relaxed);

                match packet.packet_type {
                    PacketType::Audio => {
                        if let Some(ref callback) = audio_callback {
                            callback(&packet.payload, packet.timestamp);
                        }
                    }
                    PacketType::KeepAlive => {
                        debug!("Received keep-alive");
                    }
                    PacketType::LatencyPing => {
                        // Respond with pong
                        if let Some(ping) = LatencyPing::from_bytes(&packet.payload) {
                            let pong = LatencyPong {
                                original_sent_time_us: ping.sent_time_us,
                                ping_sequence: ping.ping_sequence,
                            };
                            let pong_packet = Packet::latency_pong(
                                sequence.fetch_add(1, Ordering::Relaxed),
                                &pong,
                            );
                            if let Err(e) = transport.send_to(&pong_packet, remote_addr).await {
                                warn!("Failed to send latency pong: {}", e);
                            }
                            trace!("Responded to latency ping seq={}", ping.ping_sequence);
                        }
                    }
                    PacketType::LatencyPong => {
                        // Update RTT measurement
                        if let Some(pong) = LatencyPong::from_bytes(&packet.payload) {
                            rtt_measurement.write().process_pong(&pong);
                        }
                    }
                    PacketType::LatencyInfo => {
                        // Store peer's latency info
                        if let Some(info) = LatencyInfoMessage::from_bytes(&packet.payload) {
                            let peer_info: PeerLatencyInfo = info.into();
                            debug!(
                                "Received peer latency info: capture={:.2}ms, playback={:.2}ms, jitter={:.2}ms",
                                peer_info.capture_buffer_ms,
                                peer_info.playback_buffer_ms,
                                peer_info.jitter_buffer_ms
                            );
                            if let Some(ref callback) = latency_info_callback {
                                callback(peer_info.clone());
                            }
                            *peer_latency_info.write() = Some(peer_info);
                        }
                    }
                    _ => {}
                }
            }
        });

        self.receive_handle = Some(handle);
    }

    fn start_keepalive_loop(&mut self) {
        let transport = self.transport.clone();
        let state = self.state.clone();
        let remote_addr = self.remote_addr;
        let sequence = AtomicU32::new(0);
        let rtt_measurement = self.rtt_measurement.clone();

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(1));

            loop {
                interval.tick().await;

                let current_state = ConnectionState::from_u8(state.load(Ordering::SeqCst));
                if !current_state.can_transmit() {
                    break;
                }

                // Send keep-alive
                let packet = Packet::keep_alive(sequence.fetch_add(1, Ordering::Relaxed));
                if let Err(e) = transport.send_to(&packet, remote_addr).await {
                    warn!("Failed to send keep-alive: {}", e);
                }

                // Send latency ping for RTT measurement
                let ping = rtt_measurement.write().create_ping();
                let ping_packet =
                    Packet::latency_ping(sequence.fetch_add(1, Ordering::Relaxed), &ping);
                if let Err(e) = transport.send_to(&ping_packet, remote_addr).await {
                    warn!("Failed to send latency ping: {}", e);
                }
                trace!("Sent latency ping seq={}", ping.ping_sequence);
            }
        });

        self.keepalive_handle = Some(handle);
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.disconnect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_creation() {
        let conn = Connection::new("127.0.0.1:0").await.unwrap();
        assert!(!conn.is_connected());
        assert!(conn.local_addr().port() > 0);
    }

    #[tokio::test]
    async fn test_connection_stats_initial() {
        let conn = Connection::new("127.0.0.1:0").await.unwrap();
        let stats = conn.stats();
        assert_eq!(stats.packets_sent, 0);
        assert_eq!(stats.packets_received, 0);
    }
}
