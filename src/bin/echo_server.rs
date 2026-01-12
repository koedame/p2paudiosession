//! Echo server for testing P2P audio latency
//!
//! This server receives jamjam protocol UDP packets and echoes them back
//! to the sender after a configurable delay.
//!
//! Run with:
//!   cargo run --bin echo-server -- --port 5000 --delay 3000
//!
//! With signaling server (for GUI discovery):
//!   cargo run --bin echo-server -- --port 5000 --delay 3000 \
//!     --signaling-url ws://localhost:8080
//!
//! Environment variables:
//!   ECHO_DELAY_MS - Delay in milliseconds (default: 3000)
//!   RUST_LOG - Log level (default: info)

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::Parser;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn, Level};

use jamjam::network::{SignalingClient, SignalingConnection, SignalingMessage, UdpTransport};
use jamjam::protocol::{Packet, PacketType};

/// Echo server for jamjam P2P audio testing
#[derive(Parser, Debug)]
#[command(name = "echo-server")]
#[command(about = "Echo server for jamjam P2P audio testing")]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value = "5000")]
    port: u16,

    /// Host to bind to
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// Echo delay in milliseconds
    #[arg(short, long, default_value = "3000", env = "ECHO_DELAY_MS")]
    delay: u64,

    /// Signaling server URL (optional, enables GUI discovery)
    #[arg(long, env = "SIGNALING_URL")]
    signaling_url: Option<String>,

    /// Room name when using signaling server
    #[arg(long, default_value = "Echo Server")]
    room_name: String,

    /// Public address to advertise (defaults to UDP listen address)
    #[arg(long)]
    public_addr: Option<SocketAddr>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

/// A buffered packet waiting to be echoed
struct BufferedPacket {
    packet: Packet,
    sender: SocketAddr,
    send_at: Instant,
}

/// Echo server state
struct EchoState {
    buffer: VecDeque<BufferedPacket>,
    delay: Duration,
    packets_received: u64,
    packets_sent: u64,
}

impl EchoState {
    fn new(delay: Duration) -> Self {
        Self {
            buffer: VecDeque::new(),
            delay,
            packets_received: 0,
            packets_sent: 0,
        }
    }

    /// Add a packet to the buffer
    fn add_packet(&mut self, packet: Packet, sender: SocketAddr) {
        let send_at = Instant::now() + self.delay;
        self.buffer.push_back(BufferedPacket {
            packet,
            sender,
            send_at,
        });
        self.packets_received += 1;
    }

    /// Get packets ready to be sent
    fn get_ready_packets(&mut self) -> Vec<BufferedPacket> {
        let now = Instant::now();
        let mut ready = Vec::new();

        while let Some(buffered) = self.buffer.front() {
            if buffered.send_at <= now {
                if let Some(p) = self.buffer.pop_front() {
                    ready.push(p);
                }
            } else {
                break;
            }
        }

        ready
    }

    /// Get time until next packet is ready
    fn time_until_next(&self) -> Option<Duration> {
        self.buffer.front().map(|p| {
            let now = Instant::now();
            if p.send_at > now {
                p.send_at - now
            } else {
                Duration::ZERO
            }
        })
    }
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

    let addr = format!("{}:{}", args.host, args.port);
    let delay = Duration::from_millis(args.delay);

    info!("Echo server starting on {}", addr);
    info!("Echo delay: {}ms", args.delay);

    // Create UDP transport
    let transport = Arc::new(UdpTransport::bind(&addr).await?);
    let state = Arc::new(Mutex::new(EchoState::new(delay)));

    info!("Echo server listening on {}", transport.local_addr());

    // Connect to signaling server if URL provided
    let signaling_conn: Arc<Mutex<Option<SignalingConnection>>> = Arc::new(Mutex::new(None));
    if let Some(signaling_url) = &args.signaling_url {
        match setup_signaling(
            signaling_url,
            &args.room_name,
            args.delay,
            args.public_addr.unwrap_or(transport.local_addr()),
        )
        .await
        {
            Ok(conn) => {
                info!("Connected to signaling server: {}", signaling_url);
                *signaling_conn.lock().await = Some(conn);
            }
            Err(e) => {
                error!("Failed to connect to signaling server: {}", e);
                error!("Continuing without signaling (direct UDP only)");
            }
        }
    }

    // Spawn signaling event handler
    let signaling_conn_clone = signaling_conn.clone();
    let _signaling_handle = tokio::spawn(async move {
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
                }
                Some(SignalingMessage::PeerLeft { peer_id }) => {
                    info!("Peer left: {}", peer_id);
                }
                Some(SignalingMessage::Error { message }) => {
                    warn!("Signaling error: {}", message);
                }
                None => {
                    // No connection or connection closed
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                _ => {}
            }
        }
    });

    // Spawn the sender task
    let sender_transport = transport.clone();
    let sender_state = state.clone();
    let sender_handle = tokio::spawn(async move {
        loop {
            // Calculate sleep duration
            let sleep_duration = {
                let state = sender_state.lock().await;
                state.time_until_next().unwrap_or(Duration::from_millis(10))
            };

            tokio::time::sleep(sleep_duration).await;

            // Get and send ready packets
            let ready_packets = {
                let mut state = sender_state.lock().await;
                state.get_ready_packets()
            };

            for buffered in ready_packets {
                match sender_transport
                    .send_to(&buffered.packet, buffered.sender)
                    .await
                {
                    Ok(_) => {
                        let mut state = sender_state.lock().await;
                        state.packets_sent += 1;
                        debug!(
                            "Echoed packet seq={} to {}",
                            buffered.packet.sequence, buffered.sender
                        );
                    }
                    Err(e) => {
                        warn!("Failed to send echo to {}: {}", buffered.sender, e);
                    }
                }
            }
        }
    });

    // Main receive loop
    let mut stats_interval = tokio::time::interval(Duration::from_secs(60));
    stats_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            result = transport.recv_from() => {
                match result {
                    Ok((packet, sender)) => {
                        match packet.packet_type {
                            PacketType::Audio => {
                                let mut state = state.lock().await;
                                state.add_packet(packet.clone(), sender);
                                debug!(
                                    "Received audio packet seq={} from {} (buffered: {})",
                                    packet.sequence,
                                    sender,
                                    state.buffer.len()
                                );
                            }
                            PacketType::KeepAlive => {
                                // Echo keep-alive immediately
                                if let Err(e) = transport.send_to(&packet, sender).await {
                                    warn!("Failed to echo keep-alive to {}: {}", sender, e);
                                }
                            }
                            _ => {
                                debug!("Ignoring packet type {:?} from {}", packet.packet_type, sender);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Receive error: {}", e);
                    }
                }
            }

            _ = stats_interval.tick() => {
                let state = state.lock().await;
                info!(
                    "Stats: received={}, sent={}, buffered={}",
                    state.packets_received,
                    state.packets_sent,
                    state.buffer.len()
                );
            }

            _ = tokio::signal::ctrl_c() => {
                info!("Shutting down...");
                break;
            }
        }
    }

    sender_handle.abort();

    // Leave room on shutdown
    {
        let mut conn_guard = signaling_conn.lock().await;
        if let Some(conn) = conn_guard.as_mut() {
            let _ = conn.send(SignalingMessage::LeaveRoom).await;
        }
    }

    let state = state.lock().await;
    info!(
        "Final stats: received={}, sent={}",
        state.packets_received, state.packets_sent
    );

    Ok(())
}

/// Setup signaling connection and create room
async fn setup_signaling(
    signaling_url: &str,
    room_name: &str,
    delay_ms: u64,
    udp_addr: SocketAddr,
) -> Result<SignalingConnection, Box<dyn std::error::Error + Send + Sync>> {
    let client = SignalingClient::new(signaling_url);
    let mut conn = client.connect().await?;

    // Create room with descriptive name
    let full_room_name = format!("[BOT] {} ({}ms delay)", room_name, delay_ms);
    info!("Creating room: {}", full_room_name);

    conn.send(SignalingMessage::CreateRoom {
        room_name: full_room_name,
        password: None,
        peer_name: "Echo Bot".to_string(),
    })
    .await?;

    // Wait for RoomCreated response
    let response = conn.recv().await?;
    match response {
        SignalingMessage::RoomCreated { room_id, peer_id } => {
            info!("Room created: {} (peer_id: {})", room_id, peer_id);

            // Update peer info with UDP address
            conn.send(SignalingMessage::UpdatePeerInfo {
                public_addr: Some(udp_addr),
                local_addr: Some(udp_addr),
            })
            .await?;

            info!("Advertised UDP address: {}", udp_addr);
            Ok(conn)
        }
        SignalingMessage::Error { message } => {
            Err(format!("Failed to create room: {}", message).into())
        }
        _ => Err("Unexpected response from signaling server".into()),
    }
}
