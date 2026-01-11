//! Standalone signaling server binary
//!
//! Run with:
//!   cargo run --bin signaling-server -- --port 8080
//!
//! With TLS:
//!   cargo run --bin signaling-server -- --port 8443 --cert cert.pem --key key.pem

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, RwLock};
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info, warn, Level};
use uuid::Uuid;

use jamjam::network::{
    PeerInfo, RoomInfo, SignalingMessage, SignalingServer, MAX_PEERS_PER_ROOM,
};

/// Signaling server for jamjam P2P audio sessions
#[derive(Parser, Debug)]
#[command(name = "signaling-server")]
#[command(about = "Signaling server for jamjam P2P audio sessions")]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value = "8080")]
    port: u16,

    /// Host to bind to
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// Path to TLS certificate file (PEM format)
    #[arg(long)]
    cert: Option<PathBuf>,

    /// Path to TLS private key file (PEM format)
    #[arg(long)]
    key: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

/// Load TLS certificates from PEM file
fn load_certs(path: &PathBuf) -> Result<Vec<CertificateDer<'static>>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let certs = rustls_pemfile::certs(&mut reader).collect::<Result<Vec<_>, _>>()?;
    Ok(certs)
}

/// Load TLS private key from PEM file
fn load_key(path: &PathBuf) -> Result<PrivateKeyDer<'static>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    for item in rustls_pemfile::read_all(&mut reader) {
        match item? {
            rustls_pemfile::Item::Pkcs1Key(key) => {
                return Ok(PrivateKeyDer::Pkcs1(key));
            }
            rustls_pemfile::Item::Pkcs8Key(key) => {
                return Ok(PrivateKeyDer::Pkcs8(key));
            }
            rustls_pemfile::Item::Sec1Key(key) => {
                return Ok(PrivateKeyDer::Sec1(key));
            }
            _ => continue,
        }
    }

    Err("No private key found in file".into())
}

/// Create TLS acceptor from certificate and key files
fn create_tls_acceptor(
    cert_path: &PathBuf,
    key_path: &PathBuf,
) -> Result<TlsAcceptor, Box<dyn std::error::Error>> {
    let certs = load_certs(cert_path)?;
    let key = load_key(key_path)?;

    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    Ok(TlsAcceptor::from(Arc::new(config)))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Setup logging
    let level = if args.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };

    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .compact()
        .init();

    // Create server address
    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;

    // Check TLS configuration
    let tls_acceptor = match (&args.cert, &args.key) {
        (Some(cert), Some(key)) => {
            info!("TLS enabled with cert: {:?}, key: {:?}", cert, key);
            Some(create_tls_acceptor(cert, key)?)
        }
        (Some(_), None) | (None, Some(_)) => {
            error!("Both --cert and --key must be provided for TLS");
            return Err("TLS configuration incomplete".into());
        }
        (None, None) => {
            warn!("TLS disabled - running in plain WebSocket mode");
            warn!("For production, use --cert and --key to enable TLS");
            None
        }
    };

    info!("Signaling server starting on {}", addr);
    if tls_acceptor.is_some() {
        info!("Protocol: wss:// (WebSocket Secure)");
    } else {
        info!("Protocol: ws:// (WebSocket)");
    }

    // Run the appropriate server
    if let Some(acceptor) = tls_acceptor {
        run_tls_server(addr, acceptor).await?;
    } else {
        let server = SignalingServer::new();
        server.run(&addr.to_string()).await?;
    }

    Ok(())
}

/// Room state for TLS server
struct RoomState {
    id: String,
    name: String,
    password: Option<String>,
    peers: HashMap<Uuid, PeerInfo>,
    broadcast_tx: broadcast::Sender<SignalingMessage>,
}

/// Run the signaling server with TLS support
async fn run_tls_server(
    addr: SocketAddr,
    tls_acceptor: TlsAcceptor,
) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(addr).await?;
    info!("TLS signaling server listening on {}", addr);

    let rooms: Arc<RwLock<HashMap<String, RoomState>>> = Arc::new(RwLock::new(HashMap::new()));

    loop {
        match listener.accept().await {
            Ok((stream, peer_addr)) => {
                info!("New TLS connection from {}", peer_addr);

                let acceptor = tls_acceptor.clone();
                let rooms = rooms.clone();

                tokio::spawn(async move {
                    // Perform TLS handshake
                    let tls_stream = match acceptor.accept(stream).await {
                        Ok(s) => s,
                        Err(e) => {
                            warn!("TLS handshake failed for {}: {}", peer_addr, e);
                            return;
                        }
                    };

                    // Upgrade to WebSocket
                    let ws_stream = match tokio_tungstenite::accept_async(tls_stream).await {
                        Ok(s) => s,
                        Err(e) => {
                            warn!("WebSocket upgrade failed for {}: {}", peer_addr, e);
                            return;
                        }
                    };

                    info!("TLS WebSocket connection established with {}", peer_addr);

                    // Handle the connection
                    if let Err(e) = handle_tls_connection(ws_stream, rooms).await {
                        warn!("Connection error for {}: {}", peer_addr, e);
                    }
                });
            }
            Err(e) => {
                error!("Accept error: {}", e);
            }
        }
    }
}

/// Handle a TLS WebSocket connection
async fn handle_tls_connection<S>(
    ws_stream: tokio_tungstenite::WebSocketStream<S>,
    rooms: Arc<RwLock<HashMap<String, RoomState>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let (mut write, mut read) = ws_stream.split();
    let mut current_room: Option<String> = None;
    let mut current_peer_id: Option<Uuid> = None;
    let mut broadcast_rx: Option<broadcast::Receiver<SignalingMessage>> = None;

    loop {
        tokio::select! {
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
                                    let json = serde_json::to_string(&resp)?;
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

            broadcast = async {
                if let Some(ref mut rx) = broadcast_rx {
                    rx.recv().await.ok()
                } else {
                    std::future::pending::<Option<SignalingMessage>>().await
                }
            } => {
                if let Some(msg) = broadcast {
                    let should_send = match &msg {
                        SignalingMessage::PeerJoined { peer } => Some(peer.id) != current_peer_id,
                        SignalingMessage::PeerUpdated { peer } => Some(peer.id) != current_peer_id,
                        _ => true,
                    };

                    if should_send {
                        let json = serde_json::to_string(&msg)?;
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
    rooms: &Arc<RwLock<HashMap<String, RoomState>>>,
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
            let room_id = Uuid::new_v4().to_string()[..8].to_string();
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

            let room = RoomState {
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
                    if room.password.is_some() && room.password != password {
                        return Some(SignalingMessage::Error {
                            message: "Invalid password".to_string(),
                        });
                    }

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
                })
                .collect();

            Some(SignalingMessage::RoomList { rooms: room_list })
        }

        // Server->Client messages are ignored if received
        _ => None,
    }
}
