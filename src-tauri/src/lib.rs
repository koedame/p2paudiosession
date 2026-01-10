//! Tauri application library for jamjam

use serde::{Deserialize, Serialize};
use tauri::State;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

use jamjam::audio::{list_input_devices, list_output_devices, AudioConfig, AudioDevice};
use jamjam::network::{Session, SessionConfig};

/// Application state
pub struct AppState {
    pub session: Arc<Mutex<Option<Session>>>,
    pub config: Arc<Mutex<AudioConfig>>,
    pub audio_running: Arc<std::sync::atomic::AtomicBool>,
    pub local_monitoring: Arc<std::sync::atomic::AtomicBool>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            session: Arc::new(Mutex::new(None)),
            config: Arc::new(Mutex::new(AudioConfig::default())),
            audio_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            local_monitoring: Arc::new(std::sync::atomic::AtomicBool::new(false)),
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
    _input_device: Option<String>,
    _output_device: Option<String>,
) -> Result<(), String> {
    state.audio_running.store(true, std::sync::atomic::Ordering::SeqCst);
    info!("Audio start requested");
    Ok(())
}

#[tauri::command]
async fn cmd_stop_audio(state: State<'_, AppState>) -> Result<(), String> {
    state.audio_running.store(false, std::sync::atomic::Ordering::SeqCst);
    info!("Audio stop requested");
    Ok(())
}

#[tauri::command]
async fn cmd_set_local_monitoring(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    state.local_monitoring.store(enabled, std::sync::atomic::Ordering::SeqCst);
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

    let audio_running = state.audio_running.load(std::sync::atomic::Ordering::SeqCst);
    let local_monitoring = state.local_monitoring.load(std::sync::atomic::Ordering::SeqCst);

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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
