//! Tauri application library for jamjam

mod audio_service;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use tauri::{Manager, State};
use tokio::sync::Mutex;
use tracing::{info, warn};
use uuid::Uuid;

use jamjam::audio::{list_input_devices, list_output_devices, AudioConfig, AudioDevice};
use jamjam::network::{
    LatencyBreakdown, LocalLatencyInfo, PeerLatencyInfo, RoomInfo, Session, SessionConfig,
    SignalingClient, SignalingConnection, SignalingMessage,
};

use audio_service::AudioServiceHandle;

/// Per-peer audio settings (volume, pan, mute)
#[derive(Debug, Clone)]
pub struct PeerAudioSettings {
    pub volume: f32, // 0.0-1.0 (linear)
    pub pan: f32,    // -1.0 (left) to 1.0 (right)
    pub muted: bool,
}

impl Default for PeerAudioSettings {
    fn default() -> Self {
        Self {
            volume: 1.0,
            pan: 0.0,
            muted: false,
        }
    }
}

/// Application state
pub struct AppState {
    pub session: Arc<Mutex<Option<Session>>>,
    pub config: Arc<Mutex<AudioConfig>>,
    pub audio_service: Arc<std::sync::Mutex<AudioServiceHandle>>,
    // Signaling state
    pub signaling_conn: Arc<Mutex<Option<SignalingConnection>>>,
    pub current_room_id: Arc<Mutex<Option<String>>>,
    pub my_peer_id: Arc<Mutex<Option<Uuid>>>,
    // Per-peer audio settings
    pub peer_audio_settings: Arc<std::sync::Mutex<HashMap<Uuid, PeerAudioSettings>>>,
    // Device polling thread shutdown
    pub device_poll_shutdown: Arc<AtomicBool>,
    pub device_poll_thread: std::sync::Mutex<Option<JoinHandle<()>>>,
    // Signaling event loop task handle
    pub signaling_event_loop: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            session: Arc::new(Mutex::new(None)),
            config: Arc::new(Mutex::new(AudioConfig::default())),
            audio_service: Arc::new(std::sync::Mutex::new(AudioServiceHandle::new())),
            signaling_conn: Arc::new(Mutex::new(None)),
            current_room_id: Arc::new(Mutex::new(None)),
            my_peer_id: Arc::new(Mutex::new(None)),
            peer_audio_settings: Arc::new(std::sync::Mutex::new(HashMap::new())),
            device_poll_shutdown: Arc::new(AtomicBool::new(false)),
            device_poll_thread: std::sync::Mutex::new(None),
            signaling_event_loop: Mutex::new(None),
        }
    }
}

impl Drop for AppState {
    fn drop(&mut self) {
        // Signal device polling thread to stop
        self.device_poll_shutdown.store(true, Ordering::SeqCst);

        // Wait for thread to finish
        if let Ok(mut handle) = self.device_poll_thread.lock() {
            if let Some(h) = handle.take() {
                info!("Waiting for device polling thread to stop");
                let _ = h.join();
                info!("Device polling thread stopped");
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

impl From<AudioDevice> for DeviceInfo {
    fn from(d: AudioDevice) -> Self {
        Self {
            id: d.id.0,
            name: d.name,
            is_default: d.is_default,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfoDto {
    pub id: String,
    pub name: String,
    pub volume: f32,
    pub muted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStatus {
    pub connected: bool,
    pub room_id: Option<String>,
    pub peer_count: usize,
    pub audio_running: bool,
    pub local_monitoring: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfigDto {
    pub sample_rate: u32,
    pub channels: u16,
    pub frame_size: u32,
}

impl From<AudioConfig> for AudioConfigDto {
    fn from(c: AudioConfig) -> Self {
        Self {
            sample_rate: c.sample_rate,
            channels: c.channels,
            frame_size: c.frame_size,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomInfoDto {
    pub id: String,
    pub name: String,
    pub peer_count: usize,
    pub max_peers: usize,
    pub has_password: bool,
}

impl From<RoomInfo> for RoomInfoDto {
    fn from(r: RoomInfo) -> Self {
        Self {
            id: r.id,
            name: r.name,
            peer_count: r.peer_count,
            max_peers: r.max_peers,
            has_password: r.has_password,
        }
    }
}

/// Latency breakdown DTO for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyBreakdownDto {
    pub upstream_total_ms: f32,
    pub downstream_total_ms: f32,
    pub roundtrip_total_ms: f32,
    pub upstream: UpstreamLatencyDto,
    pub downstream: DownstreamLatencyDto,
    pub network: NetworkLatencyDto,
    pub has_peer_info: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamLatencyDto {
    pub capture_buffer_ms: f32,
    pub encode_ms: f32,
    pub network_ms: f32,
    pub peer_jitter_buffer_ms: f32,
    pub peer_decode_ms: f32,
    pub peer_playback_buffer_ms: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownstreamLatencyDto {
    pub peer_capture_buffer_ms: f32,
    pub peer_encode_ms: f32,
    pub network_ms: f32,
    pub jitter_buffer_ms: f32,
    pub decode_ms: f32,
    pub playback_buffer_ms: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkLatencyDto {
    pub rtt_ms: f32,
    pub one_way_ms: f32,
    pub jitter_ms: f32,
    pub packet_loss_rate: f32,
}

impl From<LatencyBreakdown> for LatencyBreakdownDto {
    fn from(b: LatencyBreakdown) -> Self {
        Self {
            upstream_total_ms: b.upstream_total_ms,
            downstream_total_ms: b.downstream_total_ms,
            roundtrip_total_ms: b.roundtrip_total_ms,
            upstream: UpstreamLatencyDto {
                capture_buffer_ms: b.upstream.capture_buffer_ms,
                encode_ms: b.upstream.encode_ms,
                network_ms: b.upstream.network_ms,
                peer_jitter_buffer_ms: b.upstream.peer_jitter_buffer_ms,
                peer_decode_ms: b.upstream.peer_decode_ms,
                peer_playback_buffer_ms: b.upstream.peer_playback_buffer_ms,
            },
            downstream: DownstreamLatencyDto {
                peer_capture_buffer_ms: b.downstream.peer_capture_buffer_ms,
                peer_encode_ms: b.downstream.peer_encode_ms,
                network_ms: b.downstream.network_ms,
                jitter_buffer_ms: b.downstream.jitter_buffer_ms,
                decode_ms: b.downstream.decode_ms,
                playback_buffer_ms: b.downstream.playback_buffer_ms,
            },
            network: NetworkLatencyDto {
                rtt_ms: b.network.rtt_ms,
                one_way_ms: b.network.one_way_ms,
                jitter_ms: b.network.jitter_ms,
                packet_loss_rate: b.network.packet_loss_rate,
            },
            has_peer_info: b.has_peer_info(),
        }
    }
}

#[tauri::command]
fn cmd_get_input_devices() -> Result<Vec<DeviceInfo>, String> {
    list_input_devices()
        .map(|devices| devices.into_iter().map(Into::into).collect())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_get_output_devices() -> Result<Vec<DeviceInfo>, String> {
    list_output_devices()
        .map(|devices| devices.into_iter().map(Into::into).collect())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_get_audio_config(state: State<'_, AppState>) -> Result<AudioConfigDto, String> {
    let config = state.config.lock().await;
    Ok((*config).clone().into())
}

#[tauri::command]
async fn cmd_set_audio_config(
    state: State<'_, AppState>,
    sample_rate: u32,
    channels: u16,
    frame_size: u32,
) -> Result<(), String> {
    let mut config = state.config.lock().await;
    config.sample_rate = sample_rate;
    config.channels = channels;
    config.frame_size = frame_size;
    info!("Audio config updated: {:?}", *config);
    Ok(())
}

#[tauri::command]
async fn cmd_start_audio(
    state: State<'_, AppState>,
    input_device: Option<String>,
    output_device: Option<String>,
) -> Result<(), String> {
    let config = state.config.lock().await.clone();
    let audio_service = state.audio_service.lock().map_err(|e| e.to_string())?;
    audio_service.start(input_device, output_device, config)?;
    info!("Audio started");
    Ok(())
}

#[tauri::command]
async fn cmd_stop_audio(state: State<'_, AppState>) -> Result<(), String> {
    let audio_service = state.audio_service.lock().map_err(|e| e.to_string())?;
    audio_service.stop()?;
    info!("Audio stopped");
    Ok(())
}

#[tauri::command]
async fn cmd_set_local_monitoring(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    let audio_service = state.audio_service.lock().map_err(|e| e.to_string())?;
    audio_service.set_local_monitoring(enabled)?;
    info!("Local monitoring: {}", enabled);
    Ok(())
}

#[tauri::command]
async fn cmd_set_input_device(
    state: State<'_, AppState>,
    device_id: Option<String>,
) -> Result<(), String> {
    let audio_service = state.audio_service.lock().map_err(|e| e.to_string())?;
    audio_service.set_input_device(device_id)?;
    info!("Input device changed");
    Ok(())
}

#[tauri::command]
async fn cmd_set_output_device(
    state: State<'_, AppState>,
    device_id: Option<String>,
) -> Result<(), String> {
    let audio_service = state.audio_service.lock().map_err(|e| e.to_string())?;
    audio_service.set_output_device(device_id)?;
    info!("Output device changed");
    Ok(())
}

#[tauri::command]
async fn cmd_create_session(
    state: State<'_, AppState>,
    port: u16,
) -> Result<String, String> {
    let mut session_guard = state.session.lock().await;

    if session_guard.is_some() {
        return Err("Session already exists".to_string());
    }

    let config = SessionConfig {
        local_port: port,
        max_peers: 10,
        enable_mixing: true,
    };

    let mut session = Session::new(config)
        .await
        .map_err(|e| e.to_string())?;

    // Register callback to feed received audio to AudioEngine with per-peer volume
    let peer_audio_settings = state.peer_audio_settings.clone();
    let audio_service = state.audio_service.clone();
    session.set_peer_audio_callback(move |peer_id: Uuid, samples: &[f32], _timestamp: u32| {
        // Get peer settings (volume, muted, etc.)
        let settings = peer_audio_settings
            .lock()
            .ok()
            .and_then(|s| s.get(&peer_id).cloned())
            .unwrap_or_default();

        // Skip muted peers
        if settings.muted {
            return;
        }

        // Apply volume
        let processed: Vec<f32> = samples.iter().map(|&s| s * settings.volume).collect();

        if let Ok(service) = audio_service.lock() {
            service.enqueue_remote_audio(processed);
        }
    });

    session.start();
    let local_addr = session.local_addr().to_string();

    *session_guard = Some(session);
    info!("Session created on {}", local_addr);

    Ok(local_addr)
}

#[tauri::command]
async fn cmd_leave_session(state: State<'_, AppState>) -> Result<(), String> {
    let mut session_guard = state.session.lock().await;

    if let Some(ref mut session) = *session_guard {
        session.stop();
    }

    *session_guard = None;

    // Allow time for socket resources to be fully released by OS
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    info!("Left session");
    Ok(())
}

#[tauri::command]
async fn cmd_get_session_status(state: State<'_, AppState>) -> Result<SessionStatus, String> {
    let session_guard = state.session.lock().await;

    let (connected, peer_count) = if let Some(ref session) = *session_guard {
        (true, session.peers().await.len())
    } else {
        (false, 0)
    };

    let (audio_running, local_monitoring) = {
        let audio_service = state.audio_service.lock().map_err(|e| e.to_string())?;
        (audio_service.is_running(), audio_service.is_local_monitoring())
    };

    Ok(SessionStatus {
        connected,
        room_id: None,
        peer_count,
        audio_running,
        local_monitoring,
    })
}

#[tauri::command]
async fn cmd_get_peers(state: State<'_, AppState>) -> Result<Vec<PeerInfoDto>, String> {
    let session_guard = state.session.lock().await;

    if let Some(ref session) = *session_guard {
        let peers = session.peers().await;
        Ok(peers
            .into_iter()
            .map(|p| PeerInfoDto {
                id: p.id.to_string(),
                name: p.name,
                volume: 1.0,
                muted: false,
            })
            .collect())
    } else {
        Ok(vec![])
    }
}

// ============================================
// Signaling Commands
// ============================================

#[tauri::command]
async fn cmd_connect_signaling(state: State<'_, AppState>, url: String) -> Result<(), String> {
    let mut conn_guard = state.signaling_conn.lock().await;

    if conn_guard.is_some() {
        return Err("Already connected to signaling server".to_string());
    }

    let client = SignalingClient::new(&url);
    let conn = client.connect().await.map_err(|e| e.to_string())?;

    *conn_guard = Some(conn);
    info!("Connected to signaling server: {}", url);

    Ok(())
}

#[tauri::command]
async fn cmd_disconnect_signaling(state: State<'_, AppState>) -> Result<(), String> {
    let mut conn_guard = state.signaling_conn.lock().await;
    *conn_guard = None;
    *state.current_room_id.lock().await = None;
    *state.my_peer_id.lock().await = None;
    info!("Disconnected from signaling server");
    Ok(())
}

#[tauri::command]
async fn cmd_list_rooms(state: State<'_, AppState>) -> Result<Vec<RoomInfoDto>, String> {
    let mut conn_guard = state.signaling_conn.lock().await;
    let conn = conn_guard
        .as_mut()
        .ok_or("Not connected to signaling server")?;

    conn.send(SignalingMessage::ListRooms)
        .await
        .map_err(|e| e.to_string())?;

    match conn.recv().await.map_err(|e| e.to_string())? {
        SignalingMessage::RoomList { rooms } => {
            Ok(rooms.into_iter().map(Into::into).collect())
        }
        SignalingMessage::Error { message } => Err(message),
        _ => Err("Unexpected response from server".to_string()),
    }
}

#[tauri::command]
async fn cmd_join_room(
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
    room_id: String,
    peer_name: String,
    password: Option<String>,
) -> Result<Vec<PeerInfoDto>, String> {
    // First join the room via signaling
    let peers = {
        let mut conn_guard = state.signaling_conn.lock().await;
        let conn = conn_guard
            .as_mut()
            .ok_or("Not connected to signaling server")?;

        conn.send(SignalingMessage::JoinRoom {
            room_id: room_id.clone(),
            password,
            peer_name,
        })
        .await
        .map_err(|e| e.to_string())?;

        match conn.recv().await.map_err(|e| e.to_string())? {
            SignalingMessage::RoomJoined {
                room_id: joined_room_id,
                peer_id,
                peers,
            } => {
                *state.current_room_id.lock().await = Some(joined_room_id.clone());
                *state.my_peer_id.lock().await = Some(peer_id);
                info!("Joined room {} as peer {}", joined_room_id, peer_id);
                info!("Received {} peers from signaling server:", peers.len());
                for peer in &peers {
                    info!(
                        "  Peer: {} (id={}, public_addr={:?}, local_addr={:?})",
                        peer.name, peer.id, peer.public_addr, peer.local_addr
                    );
                }
                peers
            }
            SignalingMessage::Error { message } => return Err(message),
            _ => return Err("Unexpected response from server".to_string()),
        }
    };

    // Create UDP session and add discovered peers
    {
        let mut session_guard = state.session.lock().await;

        // Clean up existing session if any
        if let Some(ref mut session) = *session_guard {
            session.stop();
        }

        let config = SessionConfig {
            local_port: 0, // Let OS assign port
            max_peers: 10,
            enable_mixing: true,
        };

        let mut session = Session::new(config).await.map_err(|e| e.to_string())?;

        // Add peers that have UDP addresses
        for peer in &peers {
            if let Some(addr) = peer.public_addr.or(peer.local_addr) {
                info!("Adding peer {} at {}", peer.name, addr);
                session
                    .add_peer(peer.clone(), addr)
                    .await
                    .map_err(|e| e.to_string())?;
            } else {
                warn!("Peer {} has no address, skipping", peer.name);
            }
        }

        // Register callback to feed received audio to AudioEngine with per-peer volume
        let peer_audio_settings = state.peer_audio_settings.clone();
        let audio_service = state.audio_service.clone();
        session.set_peer_audio_callback(move |peer_id: Uuid, samples: &[f32], _timestamp: u32| {
            // Get peer settings (volume, muted, etc.)
            let settings = peer_audio_settings
                .lock()
                .ok()
                .and_then(|s| s.get(&peer_id).cloned())
                .unwrap_or_default();

            // Skip muted peers
            if settings.muted {
                return;
            }

            // Apply volume
            let processed: Vec<f32> = samples.iter().map(|&s| s * settings.volume).collect();

            if let Ok(service) = audio_service.lock() {
                service.enqueue_remote_audio(processed);
            }
        });

        session.start();
        let local_addr = session.local_addr();
        info!("Session created on {}", local_addr);

        *session_guard = Some(session);

        // Update our peer info with UDP address
        let mut conn_guard = state.signaling_conn.lock().await;
        if let Some(conn) = conn_guard.as_mut() {
            let _ = conn
                .send(SignalingMessage::UpdatePeerInfo {
                    public_addr: Some(local_addr),
                    local_addr: Some(local_addr),
                })
                .await;
        }
    }

    // Start signaling event loop to receive peer events
    let signaling_conn_clone = state.signaling_conn.clone();
    let session_clone = state.session.clone();
    let app_handle_clone = app_handle.clone();

    let handle = tokio::spawn(async move {
        use tauri::Emitter;
        loop {
            let msg = {
                let mut conn_guard = signaling_conn_clone.lock().await;
                if let Some(conn) = conn_guard.as_mut() {
                    conn.recv().await.ok()
                } else {
                    None
                }
            };

            match msg {
                Some(SignalingMessage::PeerJoined { peer }) => {
                    info!("Peer joined: {} ({})", peer.name, peer.id);
                    if let Some(addr) = peer.public_addr.or(peer.local_addr) {
                        let mut session_guard = session_clone.lock().await;
                        if let Some(session) = session_guard.as_mut() {
                            let _ = session.add_peer(peer.clone(), addr).await;
                        }
                    }
                    let _ = app_handle_clone.emit("peer-joined", &peer);
                }
                Some(SignalingMessage::PeerUpdated { peer }) => {
                    info!("Peer updated: {} ({})", peer.name, peer.id);
                    if let Some(addr) = peer.public_addr.or(peer.local_addr) {
                        let mut session_guard = session_clone.lock().await;
                        if let Some(session) = session_guard.as_mut() {
                            let _ = session.add_peer(peer.clone(), addr).await;
                        }
                    }
                    let _ = app_handle_clone.emit("peer-updated", &peer);
                }
                Some(SignalingMessage::PeerLeft { peer_id }) => {
                    info!("Peer left: {}", peer_id);
                    let mut session_guard = session_clone.lock().await;
                    if let Some(session) = session_guard.as_mut() {
                        session.remove_peer(peer_id).await;
                    }
                    let _ = app_handle_clone.emit("peer-left", peer_id.to_string());
                }
                None => break,
                _ => {}
            }
        }
    });
    *state.signaling_event_loop.lock().await = Some(handle);

    Ok(peers
        .into_iter()
        .map(|p| PeerInfoDto {
            id: p.id.to_string(),
            name: p.name,
            volume: 1.0,
            muted: false,
        })
        .collect())
}

#[tauri::command]
async fn cmd_leave_room(state: State<'_, AppState>) -> Result<(), String> {
    // Stop signaling event loop
    if let Some(handle) = state.signaling_event_loop.lock().await.take() {
        handle.abort();
    }

    // Leave signaling room
    {
        let mut conn_guard = state.signaling_conn.lock().await;
        if let Some(conn) = conn_guard.as_mut() {
            let _ = conn.send(SignalingMessage::LeaveRoom).await;
        }
    }

    // Stop UDP session
    {
        let mut session_guard = state.session.lock().await;
        if let Some(ref mut session) = *session_guard {
            session.stop();
        }
        *session_guard = None;
    }

    *state.current_room_id.lock().await = None;
    *state.my_peer_id.lock().await = None;

    info!("Left room");
    Ok(())
}

#[tauri::command]
async fn cmd_get_signaling_status(state: State<'_, AppState>) -> Result<bool, String> {
    let conn_guard = state.signaling_conn.lock().await;
    Ok(conn_guard.is_some())
}

// ============================================
// Peer Audio Settings Commands
// ============================================

#[tauri::command]
async fn cmd_set_peer_volume(
    state: State<'_, AppState>,
    peer_id: String,
    volume: f32,
) -> Result<(), String> {
    let peer_uuid = Uuid::parse_str(&peer_id).map_err(|e| e.to_string())?;
    let mut settings = state
        .peer_audio_settings
        .lock()
        .map_err(|e| e.to_string())?;
    settings.entry(peer_uuid).or_default().volume = volume.clamp(0.0, 1.0);
    info!("Set peer {} volume to {}", peer_id, volume);
    Ok(())
}

#[tauri::command]
async fn cmd_set_peer_pan(
    state: State<'_, AppState>,
    peer_id: String,
    pan: f32,
) -> Result<(), String> {
    let peer_uuid = Uuid::parse_str(&peer_id).map_err(|e| e.to_string())?;
    let mut settings = state
        .peer_audio_settings
        .lock()
        .map_err(|e| e.to_string())?;
    settings.entry(peer_uuid).or_default().pan = pan.clamp(-1.0, 1.0);
    info!("Set peer {} pan to {}", peer_id, pan);
    Ok(())
}

#[tauri::command]
async fn cmd_set_peer_muted(
    state: State<'_, AppState>,
    peer_id: String,
    muted: bool,
) -> Result<(), String> {
    let peer_uuid = Uuid::parse_str(&peer_id).map_err(|e| e.to_string())?;
    let mut settings = state
        .peer_audio_settings
        .lock()
        .map_err(|e| e.to_string())?;
    settings.entry(peer_uuid).or_default().muted = muted;
    info!("Set peer {} muted to {}", peer_id, muted);
    Ok(())
}

/// Get local latency configuration info
/// Returns latency info based on current audio configuration
#[tauri::command]
async fn cmd_get_local_latency_info(
    state: State<'_, AppState>,
) -> Result<LocalLatencyInfoDto, String> {
    let config_guard = state.config.lock().await;

    let local_info = LocalLatencyInfo::from_audio_config(
        config_guard.frame_size,
        config_guard.sample_rate,
        "pcm", // Default codec for now
    );

    Ok(LocalLatencyInfoDto {
        capture_buffer_ms: local_info.capture_buffer_ms,
        playback_buffer_ms: local_info.playback_buffer_ms,
        encode_ms: local_info.encode_ms,
        decode_ms: local_info.decode_ms,
        jitter_buffer_ms: local_info.jitter_buffer_ms,
        frame_size: local_info.frame_size,
        sample_rate: local_info.sample_rate,
        codec: local_info.codec,
    })
}

/// Local latency info DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalLatencyInfoDto {
    pub capture_buffer_ms: f32,
    pub playback_buffer_ms: f32,
    pub encode_ms: f32,
    pub decode_ms: f32,
    pub jitter_buffer_ms: f32,
    pub frame_size: u32,
    pub sample_rate: u32,
    pub codec: String,
}

/// Get latency breakdown for all peers
/// Note: RTT/jitter values require active P2P connections with LatencyPing/Pong
#[tauri::command]
async fn cmd_get_latency_breakdown(
    state: State<'_, AppState>,
) -> Result<HashMap<String, LatencyBreakdownDto>, String> {
    let session_guard = state.session.lock().await;
    let config_guard = state.config.lock().await;

    // If no session, return empty map
    if session_guard.is_none() {
        return Ok(HashMap::new());
    }

    // Create local latency info from current config
    let local_info = LocalLatencyInfo::from_audio_config(
        config_guard.frame_size,
        config_guard.sample_rate,
        "pcm", // Default codec for now
    );

    // For now, return a default breakdown with local info only
    // Full per-peer latency tracking requires Session refactoring to expose
    // per-connection RTT/jitter measurements
    let breakdown = LatencyBreakdown::calculate(&local_info, None, 0.0, 0.0);

    let mut result = HashMap::new();
    result.insert("local".to_string(), breakdown.into());

    Ok(result)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::default())
        .setup(|app| {
            // Auto-start audio on app launch
            let state = app.state::<AppState>();
            let config = AudioConfig::default();

            // Start audio with default devices
            match state.audio_service.lock() {
                Ok(audio_service) => {
                    match audio_service.start(None, None, config) {
                        Ok(_) => info!("Audio auto-started with default devices"),
                        Err(e) => warn!("Failed to auto-start audio: {}", e),
                    }
                }
                Err(e) => warn!("Failed to lock audio service for auto-start: {}", e),
            }

            // Start background task to poll device events and emit to frontend
            let app_handle = app.handle().clone();
            let audio_service: Arc<std::sync::Mutex<audio_service::AudioServiceHandle>> =
                state.audio_service.clone();
            let shutdown_flag = state.device_poll_shutdown.clone();
            let handle = std::thread::spawn(move || {
                use tauri::Emitter;
                info!("Device polling thread started");
                while !shutdown_flag.load(Ordering::SeqCst) {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    if let Ok(service) = audio_service.lock() {
                        while let Some(event) = service.try_recv_device_event() {
                            let payload = match &event {
                                audio_service::DeviceEvent::InputDeviceDisconnected {
                                    fallback_device,
                                } => serde_json::json!({
                                    "type": "input",
                                    "fallback": fallback_device
                                }),
                                audio_service::DeviceEvent::OutputDeviceDisconnected {
                                    fallback_device,
                                } => serde_json::json!({
                                    "type": "output",
                                    "fallback": fallback_device
                                }),
                            };
                            let _ = app_handle.emit("device-disconnected", payload);
                        }
                    }
                }
                info!("Device polling thread exiting");
            });

            // Store thread handle for graceful shutdown
            if let Ok(mut thread_handle) = state.device_poll_thread.lock() {
                *thread_handle = Some(handle);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            cmd_get_input_devices,
            cmd_get_output_devices,
            cmd_get_audio_config,
            cmd_set_audio_config,
            cmd_start_audio,
            cmd_stop_audio,
            cmd_set_local_monitoring,
            cmd_set_input_device,
            cmd_set_output_device,
            cmd_create_session,
            cmd_leave_session,
            cmd_get_session_status,
            cmd_get_peers,
            // Signaling commands
            cmd_connect_signaling,
            cmd_disconnect_signaling,
            cmd_list_rooms,
            cmd_join_room,
            cmd_leave_room,
            cmd_get_signaling_status,
            // Peer audio settings commands
            cmd_set_peer_volume,
            cmd_set_peer_pan,
            cmd_set_peer_muted,
            // Latency commands
            cmd_get_local_latency_info,
            cmd_get_latency_breakdown,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
