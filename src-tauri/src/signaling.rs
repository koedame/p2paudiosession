//! Signaling IPC commands for Tauri
//!
//! Provides commands to connect to signaling servers, list rooms, and join/leave rooms.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};

use serde::Serialize;
use tokio::sync::Mutex;

use jamjam::network::{PeerInfo, RoomInfo, SignalingClient, SignalingConnection, SignalingMessage};

/// Connection ID counter
static NEXT_CONN_ID: AtomicU32 = AtomicU32::new(1);

/// Signaling state managed by Tauri
pub struct SignalingState {
    connections: Mutex<HashMap<u32, SignalingConnection>>,
}

impl SignalingState {
    pub fn new() -> Self {
        Self {
            connections: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for SignalingState {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of joining a room
#[derive(Debug, Clone, Serialize)]
pub struct JoinResult {
    pub room_id: String,
    pub peer_id: String,
    pub peers: Vec<PeerInfo>,
}

/// Connect to a signaling server
#[tauri::command]
pub async fn signaling_connect(
    url: String,
    state: tauri::State<'_, SignalingState>,
) -> Result<u32, String> {
    let client = SignalingClient::new(&url);
    let conn = client.connect().await.map_err(|e| e.to_string())?;

    let conn_id = NEXT_CONN_ID.fetch_add(1, Ordering::SeqCst);
    state.connections.lock().await.insert(conn_id, conn);

    Ok(conn_id)
}

/// Disconnect from a signaling server
#[tauri::command]
pub async fn signaling_disconnect(
    conn_id: u32,
    state: tauri::State<'_, SignalingState>,
) -> Result<(), String> {
    let mut connections = state.connections.lock().await;
    if let Some(conn) = connections.remove(&conn_id) {
        conn.close().await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// List available rooms
#[tauri::command]
pub async fn signaling_list_rooms(
    conn_id: u32,
    state: tauri::State<'_, SignalingState>,
) -> Result<Vec<RoomInfo>, String> {
    let mut connections = state.connections.lock().await;
    let conn = connections
        .get_mut(&conn_id)
        .ok_or("Connection not found")?;

    conn.send(SignalingMessage::ListRooms)
        .await
        .map_err(|e| e.to_string())?;

    match conn.recv().await.map_err(|e| e.to_string())? {
        SignalingMessage::RoomList { rooms } => Ok(rooms),
        SignalingMessage::Error { message } => Err(message),
        _ => Err("Unexpected response".to_string()),
    }
}

/// Join a room
#[tauri::command]
pub async fn signaling_join_room(
    conn_id: u32,
    room_id: String,
    peer_name: String,
    state: tauri::State<'_, SignalingState>,
) -> Result<JoinResult, String> {
    let mut connections = state.connections.lock().await;
    let conn = connections
        .get_mut(&conn_id)
        .ok_or("Connection not found")?;

    conn.send(SignalingMessage::JoinRoom {
        room_id: room_id.clone(),
        password: None,
        peer_name,
    })
    .await
    .map_err(|e| e.to_string())?;

    match conn.recv().await.map_err(|e| e.to_string())? {
        SignalingMessage::RoomJoined {
            room_id,
            peer_id,
            peers,
        } => Ok(JoinResult {
            room_id,
            peer_id: peer_id.to_string(),
            peers,
        }),
        SignalingMessage::Error { message } => Err(message),
        _ => Err("Unexpected response".to_string()),
    }
}

/// Leave the current room
#[tauri::command]
pub async fn signaling_leave_room(
    conn_id: u32,
    state: tauri::State<'_, SignalingState>,
) -> Result<(), String> {
    let mut connections = state.connections.lock().await;
    let conn = connections
        .get_mut(&conn_id)
        .ok_or("Connection not found")?;

    conn.send(SignalingMessage::LeaveRoom)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Create a new room
#[tauri::command]
pub async fn signaling_create_room(
    conn_id: u32,
    room_name: String,
    peer_name: String,
    state: tauri::State<'_, SignalingState>,
) -> Result<JoinResult, String> {
    let mut connections = state.connections.lock().await;
    let conn = connections
        .get_mut(&conn_id)
        .ok_or("Connection not found")?;

    conn.send(SignalingMessage::CreateRoom {
        room_name,
        password: None,
        peer_name,
    })
    .await
    .map_err(|e| e.to_string())?;

    match conn.recv().await.map_err(|e| e.to_string())? {
        SignalingMessage::RoomCreated { room_id, peer_id } => Ok(JoinResult {
            room_id,
            peer_id: peer_id.to_string(),
            peers: vec![],
        }),
        SignalingMessage::Error { message } => Err(message),
        _ => Err("Unexpected response".to_string()),
    }
}
