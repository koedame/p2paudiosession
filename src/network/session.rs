//! Session manager for group P2P audio sessions
//!
//! Manages multiple peer connections and audio mixing.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::error::NetworkError;
use super::signaling::PeerInfo;
use super::transport::UdpTransport;
use crate::protocol::{Packet, PacketType};

/// Session configuration
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Local UDP port (0 for auto-assign)
    pub local_port: u16,
    /// Maximum number of peers
    pub max_peers: usize,
    /// Enable audio mixing (combine all peer audio)
    pub enable_mixing: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            local_port: 0,
            max_peers: 10,
            enable_mixing: true,
        }
    }
}

/// Peer state in the session
struct Peer {
    info: PeerInfo,
    addr: SocketAddr,
    connected: AtomicBool,
    packets_received: AtomicU32,
    last_audio: Option<Vec<f32>>,
}

/// Audio callback for received audio from a peer
pub type PeerAudioCallback = Box<dyn Fn(Uuid, &[f32], u32) + Send + Sync + 'static>;

/// Mixed audio callback
pub type MixedAudioCallback = Box<dyn Fn(&[f32], u32) + Send + Sync + 'static>;

/// A multi-peer P2P audio session
pub struct Session {
    transport: Arc<UdpTransport>,
    peers: Arc<RwLock<HashMap<Uuid, Peer>>>,
    config: SessionConfig,
    running: Arc<AtomicBool>,
    sequence: AtomicU32,
    local_peer_id: Uuid,
    peer_audio_callback: Option<Arc<PeerAudioCallback>>,
    mixed_audio_callback: Option<Arc<MixedAudioCallback>>,
    receive_handle: Option<tokio::task::JoinHandle<()>>,
    /// Inner receive loop handle from UdpTransport (must be aborted to release socket)
    inner_recv_handle: Option<tokio::task::JoinHandle<()>>,
}

impl Session {
    /// Create a new session
    pub async fn new(config: SessionConfig) -> Result<Self, NetworkError> {
        let local_addr = format!("0.0.0.0:{}", config.local_port);
        let transport = UdpTransport::bind(&local_addr).await?;

        Ok(Self {
            transport: Arc::new(transport),
            peers: Arc::new(RwLock::new(HashMap::new())),
            config,
            running: Arc::new(AtomicBool::new(false)),
            sequence: AtomicU32::new(0),
            local_peer_id: Uuid::new_v4(),
            peer_audio_callback: None,
            mixed_audio_callback: None,
            receive_handle: None,
            inner_recv_handle: None,
        })
    }

    /// Get local peer ID
    pub fn local_peer_id(&self) -> Uuid {
        self.local_peer_id
    }

    /// Get local address
    pub fn local_addr(&self) -> SocketAddr {
        self.transport.local_addr()
    }

    /// Add a peer to the session
    pub async fn add_peer(&self, info: PeerInfo, addr: SocketAddr) -> Result<(), NetworkError> {
        let mut peers = self.peers.write().await;

        if peers.len() >= self.config.max_peers {
            return Err(NetworkError::SessionFull);
        }

        if peers.contains_key(&info.id) {
            return Ok(()); // Already added
        }

        info!("Adding peer {} ({}) at {}", info.name, info.id, addr);

        peers.insert(
            info.id,
            Peer {
                info,
                addr,
                connected: AtomicBool::new(true),
                packets_received: AtomicU32::new(0),
                last_audio: None,
            },
        );

        Ok(())
    }

    /// Remove a peer from the session
    pub async fn remove_peer(&self, peer_id: Uuid) {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.remove(&peer_id) {
            info!("Removed peer {} ({})", peer.info.name, peer_id);
        }
    }

    /// Get list of connected peers
    pub async fn peers(&self) -> Vec<PeerInfo> {
        let peers = self.peers.read().await;
        peers.values().map(|p| p.info.clone()).collect()
    }

    /// Set callback for individual peer audio
    pub fn set_peer_audio_callback<F>(&mut self, callback: F)
    where
        F: Fn(Uuid, &[f32], u32) + Send + Sync + 'static,
    {
        self.peer_audio_callback = Some(Arc::new(Box::new(callback)));
    }

    /// Set callback for mixed audio from all peers
    pub fn set_mixed_audio_callback<F>(&mut self, callback: F)
    where
        F: Fn(&[f32], u32) + Send + Sync + 'static,
    {
        self.mixed_audio_callback = Some(Arc::new(Box::new(callback)));
    }

    /// Start the session
    ///
    /// # Thread Safety
    /// This method takes `&mut self`, ensuring exclusive access.
    /// The `running` flag is set atomically before spawning the receive loop.
    pub fn start(&mut self) {
        if self.running.load(Ordering::SeqCst) {
            return;
        }

        self.running.store(true, Ordering::SeqCst);
        self.start_receive_loop();
        info!("Session started on {}", self.transport.local_addr());
    }

    /// Stop the session
    ///
    /// # Thread Safety
    /// This method takes `&mut self`, ensuring exclusive access.
    /// The `running` flag is set to false atomically, which signals the
    /// receive loop to terminate. The abort is a fallback.
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);

        // Abort inner receive loop first (holds socket reference)
        if let Some(handle) = self.inner_recv_handle.take() {
            handle.abort();
        }

        // Then abort outer receive loop
        if let Some(handle) = self.receive_handle.take() {
            handle.abort();
        }

        info!("Session stopped");
    }

    /// Send audio to all peers
    pub async fn broadcast_audio(&self, data: &[f32], timestamp: u32) -> Result<(), NetworkError> {
        if !self.running.load(Ordering::SeqCst) {
            return Err(NetworkError::NotConnected);
        }

        // Convert f32 samples to bytes
        let bytes: Vec<u8> = data.iter().flat_map(|&s| s.to_le_bytes()).collect();
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);
        let packet = Packet::audio(sequence, timestamp, bytes);

        // Send to all peers
        let peers = self.peers.read().await;
        for peer in peers.values() {
            if peer.connected.load(Ordering::SeqCst) {
                if let Err(e) = self.transport.send_to(&packet, peer.addr).await {
                    warn!("Failed to send to peer {}: {}", peer.info.id, e);
                }
            }
        }

        Ok(())
    }

    /// Send audio to a specific peer
    pub async fn send_audio_to(
        &self,
        peer_id: Uuid,
        data: &[f32],
        timestamp: u32,
    ) -> Result<(), NetworkError> {
        let peers = self.peers.read().await;
        let peer = peers
            .get(&peer_id)
            .ok_or_else(|| NetworkError::PeerNotFound(peer_id.to_string()))?;

        let bytes: Vec<u8> = data.iter().flat_map(|&s| s.to_le_bytes()).collect();
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);
        let packet = Packet::audio(sequence, timestamp, bytes);

        self.transport.send_to(&packet, peer.addr).await?;
        Ok(())
    }

    fn start_receive_loop(&mut self) {
        let transport = self.transport.clone();
        let peers = self.peers.clone();
        let running = self.running.clone();
        let peer_callback = self.peer_audio_callback.clone();
        let mixed_callback = self.mixed_audio_callback.clone();
        let enable_mixing = self.config.enable_mixing;

        // Start inner receive loop and store handle for cleanup
        let (mut rx, inner_handle) = transport.clone().start_receive_loop();
        self.inner_recv_handle = Some(inner_handle);

        let handle = tokio::spawn(async move {

            while let Some((packet, addr)) = rx.recv().await {
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                if packet.packet_type != PacketType::Audio {
                    continue;
                }

                // Find peer by address
                let mut peers_guard = peers.write().await;
                let peer_id = {
                    let peer = peers_guard.values().find(|p| p.addr == addr);
                    peer.map(|p| p.info.id)
                };

                if let Some(peer_id) = peer_id {
                    // Convert bytes to f32 samples
                    let samples: Vec<f32> = packet
                        .payload
                        .chunks_exact(4)
                        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                        .collect();

                    // Update peer's last audio
                    if let Some(peer) = peers_guard.get_mut(&peer_id) {
                        peer.packets_received.fetch_add(1, Ordering::Relaxed);
                        peer.last_audio = Some(samples.clone());
                    }

                    // Call per-peer callback
                    if let Some(ref callback) = peer_callback {
                        callback(peer_id, &samples, packet.timestamp);
                    }

                    // Mix audio from all peers if enabled
                    if enable_mixing {
                        if let Some(ref callback) = mixed_callback {
                            let mixed = mix_audio(&peers_guard);
                            if !mixed.is_empty() {
                                callback(&mixed, packet.timestamp);
                            }
                        }
                    }
                } else {
                    debug!("Received audio from unknown address: {}", addr);
                }
            }
        });

        self.receive_handle = Some(handle);
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Mix audio from all peers
fn mix_audio(peers: &HashMap<Uuid, Peer>) -> Vec<f32> {
    let audio_buffers: Vec<&Vec<f32>> = peers
        .values()
        .filter_map(|p| p.last_audio.as_ref())
        .collect();

    if audio_buffers.is_empty() {
        return Vec::new();
    }

    // Find the maximum length
    let max_len = audio_buffers.iter().map(|b| b.len()).max().unwrap_or(0);

    if max_len == 0 {
        return Vec::new();
    }

    // Mix all buffers
    let mut mixed = vec![0.0f32; max_len];
    let num_sources = audio_buffers.len() as f32;

    for buffer in &audio_buffers {
        for (i, &sample) in buffer.iter().enumerate() {
            mixed[i] += sample / num_sources;
        }
    }

    // Soft clip to prevent clipping
    for sample in &mut mixed {
        *sample = soft_clip(*sample);
    }

    mixed
}

/// Soft clipping function to prevent harsh distortion
fn soft_clip(x: f32) -> f32 {
    if x.abs() < 0.5 {
        x
    } else {
        x.signum() * (1.0 - (-4.0 * (x.abs() - 0.5)).exp() * 0.5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_soft_clip() {
        // Values below threshold pass through
        assert!((soft_clip(0.3) - 0.3).abs() < 0.001);
        assert!((soft_clip(-0.3) - (-0.3)).abs() < 0.001);

        // Values above threshold are compressed
        let clipped = soft_clip(1.0);
        assert!(clipped < 1.0);
        assert!(clipped > 0.5);
    }

    #[test]
    fn test_mix_audio_empty() {
        let peers: HashMap<Uuid, Peer> = HashMap::new();
        let mixed = mix_audio(&peers);
        assert!(mixed.is_empty());
    }

    #[tokio::test]
    async fn test_session_creation() {
        let config = SessionConfig::default();
        let session = Session::new(config).await.unwrap();
        assert!(session.local_addr().port() > 0);
    }
}
