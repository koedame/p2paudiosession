//! Signaling server and client for peer discovery
//!
//! Handles room creation, peer discovery, and connection coordination.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, RwLock};
use tokio_tungstenite::{accept_async, connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::error::NetworkError;

/// Maximum peers per room
pub const MAX_PEERS_PER_ROOM: usize = 10;

/// Peer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: Uuid,
    pub name: String,
    pub public_addr: Option<SocketAddr>,
    pub local_addr: Option<SocketAddr>,
}

/// Room information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomInfo {
    pub id: String,
    pub name: String,
    pub peer_count: usize,
    pub max_peers: usize,
    pub has_password: bool,
    /// 6-character invite code for easy room sharing
    pub invite_code: String,
}

/// Signaling message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum SignalingMessage {
    // Client -> Server
    CreateRoom {
        room_name: String,
        password: Option<String>,
        peer_name: String,
    },
    JoinRoom {
        room_id: String,
        password: Option<String>,
        peer_name: String,
    },
    LeaveRoom,
    UpdatePeerInfo {
        public_addr: Option<SocketAddr>,
        local_addr: Option<SocketAddr>,
    },
    ListRooms,

    // Server -> Client
    RoomCreated {
        room_id: String,
        peer_id: Uuid,
        /// 6-character invite code for easy room sharing
        invite_code: String,
    },
    RoomJoined {
        room_id: String,
        peer_id: Uuid,
        peers: Vec<PeerInfo>,
    },
    PeerJoined {
        peer: PeerInfo,
    },
    PeerLeft {
        peer_id: Uuid,
    },
    PeerUpdated {
        peer: PeerInfo,
    },
    RoomList {
        rooms: Vec<RoomInfo>,
    },
    Error {
        message: String,
    },
}

/// Room state on the server
struct Room {
    id: String,
    name: String,
    password: Option<String>,
    peers: HashMap<Uuid, PeerInfo>,
    broadcast_tx: broadcast::Sender<SignalingMessage>,
    /// 6-character invite code for easy room sharing
    invite_code: String,
}

/// Signaling server state
pub struct SignalingServer {
    rooms: Arc<RwLock<HashMap<String, Room>>>,
}

impl SignalingServer {
    /// Create a new signaling server
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start the signaling server
    pub async fn run(&self, addr: &str) -> Result<(), NetworkError> {
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| NetworkError::SignalingError(format!("Bind failed: {}", e)))?;

        info!("Signaling server listening on {}", addr);

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    info!("New signaling connection from {}", peer_addr);
                    let rooms = self.rooms.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, rooms).await {
                            warn!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Accept error: {}", e);
                }
            }
        }
    }

    /// Get list of public rooms
    pub async fn list_rooms(&self) -> Vec<RoomInfo> {
        let rooms = self.rooms.read().await;
        rooms
            .values()
            .map(|room| RoomInfo {
                id: room.id.clone(),
                name: room.name.clone(),
                peer_count: room.peers.len(),
                max_peers: MAX_PEERS_PER_ROOM,
                has_password: room.password.is_some(),
                invite_code: room.invite_code.clone(),
            })
            .collect()
    }
}

impl Default for SignalingServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle a single WebSocket connection
async fn handle_connection(
    stream: TcpStream,
    rooms: Arc<RwLock<HashMap<String, Room>>>,
) -> Result<(), NetworkError> {
    let ws_stream = accept_async(stream)
        .await
        .map_err(|e| NetworkError::SignalingError(format!("WebSocket accept failed: {}", e)))?;

    let (mut write, mut read) = ws_stream.split();
    let mut current_room: Option<String> = None;
    let mut current_peer_id: Option<Uuid> = None;
    let mut broadcast_rx: Option<broadcast::Receiver<SignalingMessage>> = None;

    loop {
        tokio::select! {
            // Handle incoming messages
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<SignalingMessage>(&text) {
                            Ok(msg) => {
                                let response = process_message(
                                    msg,
                                    &rooms,
                                    &mut current_room,
                                    &mut current_peer_id,
                                    &mut broadcast_rx,
                                ).await;

                                if let Some(resp) = response {
                                    let json = serde_json::to_string(&resp).unwrap();
                                    if write.send(Message::Text(json)).await.is_err() {
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Invalid message: {}", e);
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        break;
                    }
                    Some(Err(e)) => {
                        warn!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }

            // Handle broadcast messages
            broadcast = async {
                if let Some(ref mut rx) = broadcast_rx {
                    rx.recv().await.ok()
                } else {
                    std::future::pending::<Option<SignalingMessage>>().await
                }
            } => {
                if let Some(msg) = broadcast {
                    // Don't echo messages about ourselves
                    let should_send = match &msg {
                        SignalingMessage::PeerJoined { peer } => Some(peer.id) != current_peer_id,
                        SignalingMessage::PeerUpdated { peer } => Some(peer.id) != current_peer_id,
                        _ => true,
                    };

                    if should_send {
                        let json = serde_json::to_string(&msg).unwrap();
                        if write.send(Message::Text(json)).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    }

    // Clean up on disconnect
    if let (Some(room_id), Some(peer_id)) = (current_room, current_peer_id) {
        let mut rooms_guard = rooms.write().await;
        if let Some(room) = rooms_guard.get_mut(&room_id) {
            room.peers.remove(&peer_id);
            let _ = room
                .broadcast_tx
                .send(SignalingMessage::PeerLeft { peer_id });

            // Remove empty rooms
            if room.peers.is_empty() {
                rooms_guard.remove(&room_id);
                info!("Room {} removed (empty)", room_id);
            }
        }
    }

    Ok(())
}

/// Process a signaling message
async fn process_message(
    msg: SignalingMessage,
    rooms: &Arc<RwLock<HashMap<String, Room>>>,
    current_room: &mut Option<String>,
    current_peer_id: &mut Option<Uuid>,
    broadcast_rx: &mut Option<broadcast::Receiver<SignalingMessage>>,
) -> Option<SignalingMessage> {
    match msg {
        SignalingMessage::CreateRoom {
            room_name,
            password,
            peer_name,
        } => {
            let room_id = generate_room_id();
            let peer_id = Uuid::new_v4();
            let (tx, rx) = broadcast::channel(100);

            // Generate unique invite code (retry if collision)
            let invite_code = {
                let rooms_guard = rooms.read().await;
                let mut code = generate_invite_code();
                let mut attempts = 0;
                const MAX_ATTEMPTS: u32 = 100;
                while rooms_guard.values().any(|r| r.invite_code == code) && attempts < MAX_ATTEMPTS
                {
                    code = generate_invite_code();
                    attempts += 1;
                }
                code
            };

            let peer = PeerInfo {
                id: peer_id,
                name: peer_name,
                public_addr: None,
                local_addr: None,
            };

            let mut peers = HashMap::new();
            peers.insert(peer_id, peer);

            let room = Room {
                id: room_id.clone(),
                name: room_name,
                password,
                peers,
                broadcast_tx: tx,
                invite_code: invite_code.clone(),
            };

            rooms.write().await.insert(room_id.clone(), room);
            *current_room = Some(room_id.clone());
            *current_peer_id = Some(peer_id);
            *broadcast_rx = Some(rx);

            info!(
                "Room {} (invite: {}) created by peer {}",
                room_id, invite_code, peer_id
            );

            Some(SignalingMessage::RoomCreated {
                room_id,
                peer_id,
                invite_code,
            })
        }

        SignalingMessage::JoinRoom {
            room_id,
            password,
            peer_name,
        } => {
            let mut rooms_guard = rooms.write().await;

            // Look up room by ID or invite code
            let actual_room_id = if is_invite_code_format(&room_id) {
                // Search by invite code
                rooms_guard
                    .values()
                    .find(|r| r.invite_code == room_id)
                    .map(|r| r.id.clone())
            } else {
                // Direct room ID lookup
                if rooms_guard.contains_key(&room_id) {
                    Some(room_id.clone())
                } else {
                    None
                }
            };

            match actual_room_id.and_then(|id| rooms_guard.get_mut(&id)) {
                Some(room) => {
                    // Check password
                    if room.password.is_some() && room.password != password {
                        return Some(SignalingMessage::Error {
                            message: "Invalid password".to_string(),
                        });
                    }

                    // Check capacity
                    if room.peers.len() >= MAX_PEERS_PER_ROOM {
                        return Some(SignalingMessage::Error {
                            message: "Room is full".to_string(),
                        });
                    }

                    let peer_id = Uuid::new_v4();
                    let peer = PeerInfo {
                        id: peer_id,
                        name: peer_name,
                        public_addr: None,
                        local_addr: None,
                    };

                    let peers: Vec<PeerInfo> = room.peers.values().cloned().collect();
                    let actual_room_id = room.id.clone();
                    room.peers.insert(peer_id, peer.clone());

                    // Notify existing peers
                    let _ = room
                        .broadcast_tx
                        .send(SignalingMessage::PeerJoined { peer });

                    *current_room = Some(actual_room_id.clone());
                    *current_peer_id = Some(peer_id);
                    *broadcast_rx = Some(room.broadcast_tx.subscribe());

                    info!("Peer {} joined room {}", peer_id, actual_room_id);

                    Some(SignalingMessage::RoomJoined {
                        room_id: actual_room_id,
                        peer_id,
                        peers,
                    })
                }
                None => Some(SignalingMessage::Error {
                    message: "Room not found".to_string(),
                }),
            }
        }

        SignalingMessage::LeaveRoom => {
            if let (Some(room_id), Some(peer_id)) = (current_room.take(), current_peer_id.take()) {
                let mut rooms_guard = rooms.write().await;
                if let Some(room) = rooms_guard.get_mut(&room_id) {
                    room.peers.remove(&peer_id);
                    let _ = room
                        .broadcast_tx
                        .send(SignalingMessage::PeerLeft { peer_id });

                    if room.peers.is_empty() {
                        rooms_guard.remove(&room_id);
                        info!("Room {} removed (empty)", room_id);
                    }
                }
                *broadcast_rx = None;
            }
            None
        }

        SignalingMessage::UpdatePeerInfo {
            public_addr,
            local_addr,
        } => {
            if let (Some(room_id), Some(peer_id)) =
                (current_room.as_ref(), current_peer_id.as_ref())
            {
                let mut rooms_guard = rooms.write().await;
                if let Some(room) = rooms_guard.get_mut(room_id) {
                    if let Some(peer) = room.peers.get_mut(peer_id) {
                        peer.public_addr = public_addr;
                        peer.local_addr = local_addr;

                        let _ = room
                            .broadcast_tx
                            .send(SignalingMessage::PeerUpdated { peer: peer.clone() });
                    }
                }
            }
            None
        }

        SignalingMessage::ListRooms => {
            let rooms_guard = rooms.read().await;
            let room_list: Vec<RoomInfo> = rooms_guard
                .values()
                .map(|room| RoomInfo {
                    id: room.id.clone(),
                    name: room.name.clone(),
                    peer_count: room.peers.len(),
                    max_peers: MAX_PEERS_PER_ROOM,
                    has_password: room.password.is_some(),
                    invite_code: room.invite_code.clone(),
                })
                .collect();

            Some(SignalingMessage::RoomList { rooms: room_list })
        }

        // These are server->client messages, ignore if received
        _ => None,
    }
}

/// Generate a short room ID
fn generate_room_id() -> String {
    let id = Uuid::new_v4();
    // Take first 8 characters of UUID
    id.to_string()[..8].to_string()
}

/// Characters used for invite code generation.
/// Excludes visually confusing characters: 0, O, I, 1, L
const INVITE_CODE_CHARS: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";

/// Length of invite codes
const INVITE_CODE_LENGTH: usize = 6;

/// Generate a 6-character invite code using readable characters.
/// Uses characters A-H, J-N, P-Z, 2-9 (excludes 0, O, I, 1, L for readability).
pub fn generate_invite_code() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..INVITE_CODE_LENGTH)
        .map(|_| {
            let idx = rng.gen_range(0..INVITE_CODE_CHARS.len());
            INVITE_CODE_CHARS[idx] as char
        })
        .collect()
}

/// Check if a string matches the invite code format (6 uppercase alphanumeric characters).
pub fn is_invite_code_format(s: &str) -> bool {
    s.len() == INVITE_CODE_LENGTH && s.chars().all(|c| INVITE_CODE_CHARS.contains(&(c as u8)))
}

/// Signaling client for connecting to a signaling server
pub struct SignalingClient {
    server_url: String,
}

impl SignalingClient {
    /// Create a new signaling client
    pub fn new(server_url: &str) -> Self {
        Self {
            server_url: server_url.to_string(),
        }
    }

    /// Connect to the signaling server
    pub async fn connect(&self) -> Result<SignalingConnection, NetworkError> {
        let (ws_stream, _) = connect_async(&self.server_url)
            .await
            .map_err(|e| NetworkError::SignalingError(format!("Connect failed: {}", e)))?;

        debug!("Connected to signaling server: {}", self.server_url);

        Ok(SignalingConnection { ws_stream })
    }
}

/// An active connection to the signaling server
pub struct SignalingConnection {
    ws_stream: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
}

impl SignalingConnection {
    /// Send a message to the server
    pub async fn send(&mut self, msg: SignalingMessage) -> Result<(), NetworkError> {
        let json = serde_json::to_string(&msg)
            .map_err(|e| NetworkError::SignalingError(format!("Serialize failed: {}", e)))?;

        self.ws_stream
            .send(Message::Text(json))
            .await
            .map_err(|e| NetworkError::SignalingError(format!("Send failed: {}", e)))?;

        Ok(())
    }

    /// Receive a message from the server
    pub async fn recv(&mut self) -> Result<SignalingMessage, NetworkError> {
        loop {
            match self.ws_stream.next().await {
                Some(Ok(Message::Text(text))) => {
                    return serde_json::from_str(&text).map_err(|e| {
                        NetworkError::SignalingError(format!("Deserialize failed: {}", e))
                    });
                }
                Some(Ok(Message::Close(_))) | None => {
                    return Err(NetworkError::SignalingError(
                        "Connection closed".to_string(),
                    ));
                }
                Some(Err(e)) => {
                    return Err(NetworkError::SignalingError(format!(
                        "Receive failed: {}",
                        e
                    )));
                }
                _ => continue,
            }
        }
    }

    /// Close the connection
    pub async fn close(mut self) -> Result<(), NetworkError> {
        self.ws_stream
            .close(None)
            .await
            .map_err(|e| NetworkError::SignalingError(format!("Close failed: {}", e)))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_room_id() {
        let id = generate_room_id();
        assert_eq!(id.len(), 8);
    }

    #[test]
    fn test_signaling_message_serialize() {
        let msg = SignalingMessage::CreateRoom {
            room_name: "Test Room".to_string(),
            password: None,
            peer_name: "Alice".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: SignalingMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            SignalingMessage::CreateRoom {
                room_name,
                password,
                peer_name,
            } => {
                assert_eq!(room_name, "Test Room");
                assert!(password.is_none());
                assert_eq!(peer_name, "Alice");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_generate_invite_code_length() {
        let code = generate_invite_code();
        assert_eq!(code.len(), INVITE_CODE_LENGTH);
    }

    #[test]
    fn test_generate_invite_code_valid_chars() {
        // Generate multiple codes to test character validity
        for _ in 0..100 {
            let code = generate_invite_code();
            for c in code.chars() {
                assert!(
                    INVITE_CODE_CHARS.contains(&(c as u8)),
                    "Invalid character '{}' in invite code",
                    c
                );
            }
        }
    }

    #[test]
    fn test_generate_invite_code_excludes_confusing_chars() {
        // Generate many codes and verify excluded characters never appear
        let excluded_chars = ['0', 'O', 'I', '1', 'L'];
        for _ in 0..1000 {
            let code = generate_invite_code();
            for c in excluded_chars {
                assert!(
                    !code.contains(c),
                    "Invite code '{}' contains excluded character '{}'",
                    code,
                    c
                );
            }
        }
    }

    #[test]
    fn test_generate_invite_code_uniqueness() {
        // Generate many codes and check for uniqueness (probabilistic test)
        use std::collections::HashSet;
        let mut codes = HashSet::new();
        for _ in 0..1000 {
            let code = generate_invite_code();
            codes.insert(code);
        }
        // With 29^6 possible codes (~594M), 1000 codes should all be unique
        assert_eq!(codes.len(), 1000);
    }

    #[test]
    fn test_is_invite_code_format_valid() {
        assert!(is_invite_code_format("ABC234"));
        assert!(is_invite_code_format("HJKMNP"));
        assert!(is_invite_code_format("QRSTUV"));
        assert!(is_invite_code_format("WXY789"));
    }

    #[test]
    fn test_is_invite_code_format_invalid_length() {
        assert!(!is_invite_code_format("ABC23")); // Too short
        assert!(!is_invite_code_format("ABC2345")); // Too long
        assert!(!is_invite_code_format("")); // Empty
    }

    #[test]
    fn test_is_invite_code_format_invalid_chars() {
        assert!(!is_invite_code_format("ABC230")); // Contains '0'
        assert!(!is_invite_code_format("ABCDE1")); // Contains '1'
        assert!(!is_invite_code_format("ABCDEO")); // Contains 'O'
        assert!(!is_invite_code_format("ABCDEI")); // Contains 'I'
        assert!(!is_invite_code_format("ABCDEL")); // Contains 'L'
        assert!(!is_invite_code_format("abc234")); // Lowercase
    }

    #[test]
    fn test_is_invite_code_format_uuid_like_strings() {
        // UUID-like strings should not match invite code format
        assert!(!is_invite_code_format("a1b2c3d4")); // 8-char UUID prefix
        assert!(!is_invite_code_format("a1b2c3d4-e5f6"));
    }
}
