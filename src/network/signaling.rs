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
            let _ = room.broadcast_tx.send(SignalingMessage::PeerLeft { peer_id });

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
            };

            rooms.write().await.insert(room_id.clone(), room);
            *current_room = Some(room_id.clone());
            *current_peer_id = Some(peer_id);
            *broadcast_rx = Some(rx);

            info!("Room {} created by peer {}", room_id, peer_id);

            Some(SignalingMessage::RoomCreated { room_id, peer_id })
        }

        SignalingMessage::JoinRoom {
            room_id,
            password,
            peer_name,
        } => {
            let mut rooms_guard = rooms.write().await;

            match rooms_guard.get_mut(&room_id) {
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
                    room.peers.insert(peer_id, peer.clone());

                    // Notify existing peers
                    let _ = room
                        .broadcast_tx
                        .send(SignalingMessage::PeerJoined { peer });

                    *current_room = Some(room_id.clone());
                    *current_peer_id = Some(peer_id);
                    *broadcast_rx = Some(room.broadcast_tx.subscribe());

                    info!("Peer {} joined room {}", peer_id, room_id);

                    Some(SignalingMessage::RoomJoined {
                        room_id,
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
                    let _ = room.broadcast_tx.send(SignalingMessage::PeerLeft { peer_id });

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
            if let (Some(room_id), Some(peer_id)) = (current_room.as_ref(), current_peer_id.as_ref())
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
                    return Err(NetworkError::SignalingError("Connection closed".to_string()));
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
}
