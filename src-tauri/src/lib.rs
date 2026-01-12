//! Tauri application library for jamjam

mod audio_service;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;
use tracing::{info, warn};
use uuid::Uuid;

use jamjam::audio::{list_input_devices, list_output_devices, AudioConfig, AudioDevice};
use jamjam::network::{
    RoomInfo, Session, SessionConfig, SignalingClient, SignalingConnection, SignalingMessage,
};

use audio_service::AudioServiceHandle;

/// Application state
pub struct AppState {
    pub session: Arc<Mutex<Option<Session>>>,
    pub config: Arc<Mutex<AudioConfig>>,
    pub audio_service: Arc<std::sync::Mutex<AudioServiceHandle>>,
    // Signaling state
    pub signaling_conn: Arc<Mutex<Option<SignalingConnection>>>,
    pub current_room_id: Arc<Mutex<Option<String>>>,
    pub my_peer_id: Arc<Mutex<Option<Uuid>>>,
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

#[tauri::command]
fn cmd_get_input_devices() -> Vec<DeviceInfo> {
    list_input_devices().into_iter().map(Into::into).collect()
}

#[tauri::command]
fn cmd_get_output_devices() -> Vec<DeviceInfo> {
    list_output_devices().into_iter().map(Into::into).collect()
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

    // Register callback to feed received audio to AudioEngine
    let audio_service = state.audio_service.clone();
    session.set_mixed_audio_callback(move |samples: &[f32], _timestamp: u32| {
        if let Ok(service) = audio_service.lock() {
            service.enqueue_remote_audio(samples.to_vec());
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

        // Register callback to feed received audio to AudioEngine
        let audio_service = state.audio_service.clone();
        session.set_mixed_audio_callback(move |samples: &[f32], _timestamp: u32| {
            if let Ok(service) = audio_service.lock() {
                service.enqueue_remote_audio(samples.to_vec());
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            cmd_get_input_devices,
            cmd_get_output_devices,
            cmd_get_audio_config,
            cmd_set_audio_config,
            cmd_start_audio,
            cmd_stop_audio,
            cmd_set_local_monitoring,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
