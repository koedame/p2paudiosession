//! Connection management for P2P audio streaming

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::time::interval;
use tracing::{debug, info, warn};

use crate::protocol::{Packet, PacketType};

use super::error::NetworkError;
use super::transport::UdpTransport;

/// Connection statistics
#[derive(Debug, Clone, Default)]
pub struct ConnectionStats {
    /// Round-trip time in milliseconds
    pub rtt_ms: f32,
    /// Packet loss rate (0.0 - 1.0)
    pub packet_loss_rate: f32,
    /// Jitter in milliseconds
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

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
}

/// Callback for received audio data
pub type AudioCallback = Box<dyn Fn(&[u8], u32) + Send + Sync + 'static>;

/// A P2P connection to a remote peer
pub struct Connection {
    transport: Arc<UdpTransport>,
    remote_addr: SocketAddr,
    state: Arc<AtomicBool>, // true = connected
    sequence: AtomicU32,
    packets_sent: Arc<AtomicU64>,
    packets_received: Arc<AtomicU64>,
    bytes_sent: Arc<AtomicU64>,
    bytes_received: Arc<AtomicU64>,
    last_received: Arc<std::sync::Mutex<Instant>>,
    audio_callback: Option<Arc<AudioCallback>>,
    receive_handle: Option<tokio::task::JoinHandle<()>>,
    keepalive_handle: Option<tokio::task::JoinHandle<()>>,
}

impl Connection {
    /// Create a new connection (not yet connected)
    pub async fn new(local_addr: &str) -> Result<Self, NetworkError> {
        let transport = UdpTransport::bind(local_addr).await?;

        Ok(Self {
            transport: Arc::new(transport),
            remote_addr: "0.0.0.0:0".parse().unwrap(),
            state: Arc::new(AtomicBool::new(false)),
            sequence: AtomicU32::new(0),
            packets_sent: Arc::new(AtomicU64::new(0)),
            packets_received: Arc::new(AtomicU64::new(0)),
            bytes_sent: Arc::new(AtomicU64::new(0)),
            bytes_received: Arc::new(AtomicU64::new(0)),
            last_received: Arc::new(std::sync::Mutex::new(Instant::now())),
            audio_callback: None,
            receive_handle: None,
            keepalive_handle: None,
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
        info!("Connecting to {}", remote_addr);

        // Send initial keep-alive to establish connection
        let packet = Packet::keep_alive(self.next_sequence());
        self.transport.send_to(&packet, remote_addr).await?;

        self.state.store(true, Ordering::SeqCst);
        self.start_receive_loop();
        self.start_keepalive_loop();

        info!("Connected to {}", remote_addr);
        Ok(())
    }

    /// Disconnect from the remote peer
    pub fn disconnect(&mut self) {
        self.state.store(false, Ordering::SeqCst);

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
        self.state.load(Ordering::SeqCst)
    }

    /// Set callback for received audio data
    pub fn set_audio_callback<F>(&mut self, callback: F)
    where
        F: Fn(&[u8], u32) + Send + Sync + 'static,
    {
        self.audio_callback = Some(Arc::new(Box::new(callback)));
    }

    /// Send audio data to the remote peer
    pub async fn send_audio(&self, data: &[f32], timestamp: u32) -> Result<(), NetworkError> {
        if !self.is_connected() {
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
        ConnectionStats {
            rtt_ms: 0.0,           // TODO: Implement RTT measurement
            packet_loss_rate: 0.0, // TODO: Implement packet loss tracking
            jitter_ms: 0.0,        // TODO: Implement jitter measurement
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            packets_sent: self.packets_sent.load(Ordering::Relaxed),
            packets_received: self.packets_received.load(Ordering::Relaxed),
            uptime_seconds: 0, // TODO: Track connection start time
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

        let handle = tokio::spawn(async move {
            let (mut rx, _recv_handle) = transport.clone().start_receive_loop();

            while let Some((packet, _addr)) = rx.recv().await {
                if !state.load(Ordering::SeqCst) {
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

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(1));

            loop {
                interval.tick().await;

                if !state.load(Ordering::SeqCst) {
                    break;
                }

                let packet = Packet::keep_alive(sequence.fetch_add(1, Ordering::Relaxed));
                if let Err(e) = transport.send_to(&packet, remote_addr).await {
                    warn!("Failed to send keep-alive: {}", e);
                }
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
