//! Signaling IPC commands for Tauri
//!
//! Provides commands to connect to signaling servers, list rooms, and join/leave rooms.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};

use serde::Serialize;
use tokio::sync::Mutex;

use jamjam::network::{PeerInfo, RoomInfo, SignalingClient, SignalingConnection, SignalingMessage};
use uuid::Uuid;

/// Connection ID counter
static NEXT_CONN_ID: AtomicU32 = AtomicU32::new(1);

/// Chat message for UI display
#[derive(Debug, Clone, Serialize)]
pub struct ChatMessage {
    pub id: String,
    pub sender_id: String,
    pub sender_name: String,
    pub content: String,
    pub timestamp: u64,
    /// True for system messages (join/leave notifications)
    pub is_system: bool,
}

/// Current room state
struct RoomState {
    _room_id: String,
    peer_id: String,
    peer_name: String,
    chat_messages: Vec<ChatMessage>,
}

/// Signaling state managed by Tauri
pub struct SignalingState {
    connections: Mutex<HashMap<u32, SignalingConnection>>,
    room_state: Mutex<Option<RoomState>>,
}

impl SignalingState {
    pub fn new() -> Self {
        Self {
            connections: Mutex::new(HashMap::new()),
            room_state: Mutex::new(None),
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
    pub invite_code: String,
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
        peer_name: peer_name.clone(),
    })
    .await
    .map_err(|e| e.to_string())?;

    match conn.recv().await.map_err(|e| e.to_string())? {
        SignalingMessage::RoomJoined {
            room_id,
            peer_id,
            peers,
        } => {
            // Store room state for chat
            let peer_id_str = peer_id.to_string();
            let mut room_state = state.room_state.lock().await;
            *room_state = Some(RoomState {
                _room_id: room_id.clone(),
                peer_id: peer_id_str.clone(),
                peer_name,
                chat_messages: vec![],
            });

            Ok(JoinResult {
                room_id,
                peer_id: peer_id_str,
                invite_code: String::new(), // Not returned when joining existing room
                peers,
            })
        }
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
        peer_name: peer_name.clone(),
    })
    .await
    .map_err(|e| e.to_string())?;

    match conn.recv().await.map_err(|e| e.to_string())? {
        SignalingMessage::RoomCreated {
            room_id,
            peer_id,
            invite_code,
        } => {
            // Store room state for chat
            let peer_id_str = peer_id.to_string();
            let mut room_state = state.room_state.lock().await;
            *room_state = Some(RoomState {
                _room_id: room_id.clone(),
                peer_id: peer_id_str.clone(),
                peer_name,
                chat_messages: vec![],
            });

            Ok(JoinResult {
                room_id,
                peer_id: peer_id_str,
                invite_code,
                peers: vec![],
            })
        }
        SignalingMessage::Error { message } => Err(message),
        _ => Err("Unexpected response".to_string()),
    }
}

/// Get current timestamp in milliseconds
fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Send a chat message
#[tauri::command]
pub async fn signaling_send_chat(
    conn_id: u32,
    content: String,
    state: tauri::State<'_, SignalingState>,
) -> Result<(), String> {
    // Get sender info from room state
    let room_state_guard = state.room_state.lock().await;
    let room_state = room_state_guard.as_ref().ok_or("Not in a room")?;
    let sender_id = room_state.peer_id.clone();
    let sender_name = room_state.peer_name.clone();
    drop(room_state_guard);

    let mut connections = state.connections.lock().await;
    let conn = connections
        .get_mut(&conn_id)
        .ok_or("Connection not found")?;

    let timestamp = current_timestamp();

    conn.send(SignalingMessage::ChatMessage {
        sender_id,
        sender_name,
        content,
        timestamp,
    })
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Get chat messages (for polling)
#[tauri::command]
pub async fn signaling_get_chat_messages(
    since_timestamp: Option<u64>,
    state: tauri::State<'_, SignalingState>,
) -> Result<Vec<ChatMessage>, String> {
    let room_state_guard = state.room_state.lock().await;
    let room_state = room_state_guard.as_ref().ok_or("Not in a room")?;

    let messages = if let Some(since) = since_timestamp {
        room_state
            .chat_messages
            .iter()
            .filter(|m| m.timestamp > since)
            .cloned()
            .collect()
    } else {
        room_state.chat_messages.clone()
    };

    Ok(messages)
}

/// Signaling event for the UI
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum SignalingEvent {
    /// A peer joined the room
    PeerJoined { peer: PeerInfo },
    /// A peer left the room
    PeerLeft { peer_id: String },
    /// A peer's info was updated
    PeerUpdated { peer: PeerInfo },
    /// A chat message was received
    ChatMessageReceived { message: ChatMessage },
}

/// Poll for signaling events (peer join/leave, chat messages)
/// Returns pending events and clears them from the queue
#[tauri::command]
pub async fn signaling_poll_events(
    conn_id: u32,
    state: tauri::State<'_, SignalingState>,
) -> Result<Vec<SignalingEvent>, String> {
    use tokio::time::{timeout, Duration};

    let mut events = Vec::new();

    // Try to receive messages with a short timeout
    let mut connections = state.connections.lock().await;
    let conn = match connections.get_mut(&conn_id) {
        Some(c) => c,
        None => return Ok(events), // No connection, return empty
    };

    // Poll with 50ms timeout to avoid blocking too long
    loop {
        match timeout(Duration::from_millis(50), conn.recv()).await {
            Ok(Ok(msg)) => {
                match msg {
                    SignalingMessage::PeerJoined { peer } => {
                        // Add system message for join
                        let join_msg = format!("{} が参加しました", peer.name);
                        let mut room_state = state.room_state.lock().await;
                        if let Some(ref mut rs) = *room_state {
                            rs.chat_messages.push(ChatMessage {
                                id: Uuid::new_v4().to_string(),
                                sender_id: String::new(),
                                sender_name: String::new(),
                                content: join_msg,
                                timestamp: current_timestamp(),
                                is_system: true,
                            });
                        }
                        drop(room_state);

                        events.push(SignalingEvent::PeerJoined { peer });
                    }
                    SignalingMessage::PeerLeft { peer_id } => {
                        // Add system message for leave
                        let leave_msg = format!("ユーザーが退出しました");
                        let mut room_state = state.room_state.lock().await;
                        if let Some(ref mut rs) = *room_state {
                            rs.chat_messages.push(ChatMessage {
                                id: Uuid::new_v4().to_string(),
                                sender_id: String::new(),
                                sender_name: String::new(),
                                content: leave_msg,
                                timestamp: current_timestamp(),
                                is_system: true,
                            });
                        }
                        drop(room_state);

                        events.push(SignalingEvent::PeerLeft {
                            peer_id: peer_id.to_string(),
                        });
                    }
                    SignalingMessage::PeerUpdated { peer } => {
                        events.push(SignalingEvent::PeerUpdated { peer });
                    }
                    SignalingMessage::ChatMessage {
                        sender_id,
                        sender_name,
                        content,
                        timestamp,
                    } => {
                        let chat_msg = ChatMessage {
                            id: Uuid::new_v4().to_string(),
                            sender_id: sender_id.clone(),
                            sender_name: sender_name.clone(),
                            content: content.clone(),
                            timestamp,
                            is_system: false,
                        };

                        // Store in room state
                        let mut room_state = state.room_state.lock().await;
                        if let Some(ref mut rs) = *room_state {
                            rs.chat_messages.push(chat_msg.clone());
                        }
                        drop(room_state);

                        events.push(SignalingEvent::ChatMessageReceived { message: chat_msg });
                    }
                    _ => {
                        // Ignore other message types during polling
                    }
                }
            }
            Ok(Err(_)) => {
                // Connection error, stop polling
                break;
            }
            Err(_) => {
                // Timeout, no more messages
                break;
            }
        }
    }

    Ok(events)
}
