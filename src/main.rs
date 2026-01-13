//! jamjam - Low-latency P2P audio communication for musicians

use std::sync::Arc;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

use jamjam::audio::{list_input_devices, list_output_devices, AudioConfig, AudioEngine};
use jamjam::network::{Connection, SignalingClient, SignalingMessage};

#[derive(Parser)]
#[command(name = "jamjam")]
#[command(about = "Low-latency P2P audio communication for musicians")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// List available audio devices
    Devices {
        #[command(subcommand)]
        action: DevicesAction,
    },

    /// Host a session
    Host {
        /// Port to listen on
        #[arg(short, long, default_value = "5000")]
        port: u16,

        /// Sample rate in Hz
        #[arg(long, default_value = "48000")]
        sample_rate: u32,

        /// Frame size in samples
        #[arg(long, default_value = "128")]
        frame_size: u32,
    },

    /// Join a session
    Join {
        /// Remote address (IP:PORT)
        address: String,

        /// Sample rate in Hz
        #[arg(long, default_value = "48000")]
        sample_rate: u32,

        /// Frame size in samples
        #[arg(long, default_value = "128")]
        frame_size: u32,
    },

    /// List rooms on signaling server
    Rooms {
        /// Signaling server URL (e.g., wss://example.com)
        #[arg(short, long)]
        server: String,
    },

    /// Join a room via signaling server
    JoinRoom {
        /// Signaling server URL (e.g., wss://example.com)
        #[arg(short, long)]
        server: String,

        /// Room ID to join
        #[arg(short, long)]
        room: String,

        /// Your display name
        #[arg(short, long, default_value = "CLI User")]
        name: String,

        /// Sample rate in Hz
        #[arg(long, default_value = "48000")]
        sample_rate: u32,

        /// Frame size in samples
        #[arg(long, default_value = "128")]
        frame_size: u32,
    },
}

#[derive(Subcommand)]
enum DevicesAction {
    /// List all devices
    List,
}

fn setup_logging(verbose: bool) {
    let level = if verbose { Level::DEBUG } else { Level::INFO };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");
}

fn list_devices() {
    println!("Input devices:");
    for device in list_input_devices() {
        let default_marker = if device.is_default { " (default)" } else { "" };
        println!("  - {}{}", device.name, default_marker);
    }

    println!("\nOutput devices:");
    for device in list_output_devices() {
        let default_marker = if device.is_default { " (default)" } else { "" };
        println!("  - {}{}", device.name, default_marker);
    }
}

async fn run_host(port: u16, sample_rate: u32, frame_size: u32) -> Result<()> {
    let config = AudioConfig {
        sample_rate,
        channels: 1,
        frame_size,
    };

    info!("Starting host on port {}", port);
    info!("Audio config: {:?}", config);

    let connection = Connection::new(&format!("0.0.0.0:{}", port)).await?;
    info!("Listening on {}", connection.local_addr());

    let mut audio_engine = AudioEngine::new(config.clone());

    // Start capture (for now just log, full implementation would track connected peers)
    audio_engine.start_capture(None, move |samples, timestamp| {
        if timestamp % 48000 == 0 {
            tracing::debug!(
                "Captured {} samples at timestamp {}",
                samples.len(),
                timestamp
            );
        }
    })?;

    audio_engine.start_playback(None)?;

    println!("\nHost started. Listening on port {}.", port);
    println!("Press Ctrl+C to stop.\n");

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;

    info!("Shutting down...");
    audio_engine.stop_capture();
    audio_engine.stop_playback();

    let stats = connection.stats();
    println!("\nSession statistics:");
    println!("  Packets sent: {}", stats.packets_sent);
    println!("  Packets received: {}", stats.packets_received);
    println!("  Bytes sent: {}", stats.bytes_sent);
    println!("  Bytes received: {}", stats.bytes_received);

    Ok(())
}

async fn run_join(address: String, sample_rate: u32, frame_size: u32) -> Result<()> {
    let config = AudioConfig {
        sample_rate,
        channels: 1,
        frame_size,
    };

    info!("Joining session at {}", address);
    info!("Audio config: {:?}", config);

    let remote_addr: std::net::SocketAddr = address.parse()?;
    let mut connection = Connection::new("0.0.0.0:0").await?;

    let mut audio_engine = AudioEngine::new(config.clone());

    // Create channels for audio data
    // tx_capture: capture callback -> send task
    let (tx_capture, mut rx_capture) = tokio::sync::mpsc::channel::<(Vec<f32>, u32)>(64);
    // tx_playback: receive callback -> playback task
    let (tx_playback, mut rx_playback) = tokio::sync::mpsc::channel::<Vec<f32>>(64);

    // Start audio capture
    audio_engine.start_capture(None, move |samples, timestamp| {
        let _ = tx_capture.try_send((samples.to_vec(), timestamp as u32));
    })?;

    audio_engine.start_playback(None)?;

    // Set up audio receive callback BEFORE connect
    // (connect starts receive loop which clones the callback)
    connection.set_audio_callback(move |data, _timestamp| {
        // Convert bytes back to f32 samples
        let samples: Vec<f32> = data
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();

        let _ = tx_playback.try_send(samples);
    });

    // Connect to remote (starts receive loop)
    connection.connect(remote_addr).await?;

    println!("\nConnected to {}. Session active.", address);
    println!("Press Ctrl+C to stop.\n");

    // Spawn task to send captured audio
    let connection_arc = Arc::new(tokio::sync::Mutex::new(connection));
    let connection_for_send = connection_arc.clone();

    let send_task = tokio::spawn(async move {
        let mut packet_count = 0u64;
        while let Some((samples, timestamp)) = rx_capture.recv().await {
            let conn = connection_for_send.lock().await;
            if conn.is_connected() {
                if let Err(e) = conn.send_audio(&samples, timestamp).await {
                    tracing::warn!("Failed to send audio: {}", e);
                } else {
                    packet_count += 1;
                    if packet_count % 100 == 0 {
                        tracing::debug!("Sent {} audio packets", packet_count);
                    }
                }
            }
        }
    });

    // Process received audio on main thread using select
    let mut received_count = 0u64;
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Shutting down...");
                break;
            }
            Some(samples) = rx_playback.recv() => {
                audio_engine.enqueue_playback(&samples);
                received_count += 1;
                if received_count % 100 == 0 {
                    tracing::debug!("Received {} audio packets for playback", received_count);
                }
            }
        }
    }

    send_task.abort();

    let stats = {
        let mut conn = connection_arc.lock().await;
        let stats = conn.stats();
        conn.disconnect();
        stats
    };

    audio_engine.stop_capture();
    audio_engine.stop_playback();

    println!("\nSession statistics:");
    println!("  Packets sent: {}", stats.packets_sent);
    println!("  Packets received: {}", stats.packets_received);
    println!("  Bytes sent: {}", stats.bytes_sent);
    println!("  Bytes received: {}", stats.bytes_received);

    Ok(())
}

async fn run_rooms(server: String) -> Result<()> {
    info!("Connecting to signaling server: {}", server);

    let client = SignalingClient::new(&server);
    let mut conn = client.connect().await?;

    info!("Connected, listing rooms...");

    conn.send(SignalingMessage::ListRooms).await?;

    match conn.recv().await? {
        SignalingMessage::RoomList { rooms } => {
            if rooms.is_empty() {
                println!("No rooms available.");
            } else {
                println!("Available rooms:");
                for room in rooms {
                    let password_str = if room.has_password { " (password protected)" } else { "" };
                    println!(
                        "  {} - {} ({}/{} peers){}",
                        room.id, room.name, room.peer_count, room.max_peers, password_str
                    );
                }
            }
        }
        SignalingMessage::Error { message } => {
            anyhow::bail!("Server error: {}", message);
        }
        _ => {
            anyhow::bail!("Unexpected response from server");
        }
    }

    Ok(())
}

async fn run_join_room(
    server: String,
    room_id: String,
    peer_name: String,
    sample_rate: u32,
    frame_size: u32,
) -> Result<()> {
    let config = AudioConfig {
        sample_rate,
        channels: 1,
        frame_size,
    };

    info!("Connecting to signaling server: {}", server);

    let client = SignalingClient::new(&server);
    let mut conn = client.connect().await?;

    info!("Connected, joining room {}...", room_id);

    conn.send(SignalingMessage::JoinRoom {
        room_id: room_id.clone(),
        password: None,
        peer_name: peer_name.clone(),
    })
    .await?;

    let peers = match conn.recv().await? {
        SignalingMessage::RoomJoined {
            room_id: joined_room_id,
            peer_id,
            peers,
        } => {
            info!("Joined room {} as peer {}", joined_room_id, peer_id);
            println!("\nJoined room: {}", joined_room_id);
            println!("Your peer ID: {}", peer_id);
            println!("\nPeers in room ({}):", peers.len());
            for peer in &peers {
                println!(
                    "  - {} (id: {}, addr: {:?})",
                    peer.name, peer.id, peer.public_addr
                );
            }
            peers
        }
        SignalingMessage::Error { message } => {
            anyhow::bail!("Failed to join room: {}", message);
        }
        _ => {
            anyhow::bail!("Unexpected response from server");
        }
    };

    // Find a peer with an address to connect to
    let target_peer = peers.iter().find(|p| p.public_addr.is_some());

    if let Some(peer) = target_peer {
        let remote_addr = peer.public_addr.unwrap();
        println!("\nConnecting to peer {} at {}...", peer.name, remote_addr);

        let mut connection = Connection::new("0.0.0.0:0").await?;
        let local_addr = connection.local_addr();
        info!("Local UDP socket: {}", local_addr);

        // Update our peer info with UDP address
        conn.send(SignalingMessage::UpdatePeerInfo {
            public_addr: Some(local_addr),
            local_addr: Some(local_addr),
        })
        .await?;

        let mut audio_engine = AudioEngine::new(config.clone());

        // Create channels for audio data
        let (tx_capture, mut rx_capture) = tokio::sync::mpsc::channel::<(Vec<f32>, u32)>(64);
        let (tx_playback, mut rx_playback) = tokio::sync::mpsc::channel::<Vec<f32>>(64);

        // Start audio capture
        audio_engine.start_capture(None, move |samples, timestamp| {
            let _ = tx_capture.try_send((samples.to_vec(), timestamp as u32));
        })?;

        audio_engine.start_playback(None)?;

        // Set up audio receive callback BEFORE connect
        // (connect starts receive loop which clones the callback)
        connection.set_audio_callback(move |data, _timestamp| {
            let samples: Vec<f32> = data
                .chunks_exact(4)
                .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect();
            let _ = tx_playback.try_send(samples);
        });

        // Connect to remote peer (starts receive loop)
        connection.connect(remote_addr).await?;

        println!("\nConnected to peer. Session active.");
        println!("Audio config: {:?}", config);
        println!("Press Ctrl+C to stop.\n");

        let connection_arc = Arc::new(tokio::sync::Mutex::new(connection));
        let connection_for_send = connection_arc.clone();

        // Spawn task to send captured audio
        let send_task = tokio::spawn(async move {
            let mut packet_count = 0u64;
            while let Some((samples, timestamp)) = rx_capture.recv().await {
                let conn = connection_for_send.lock().await;
                if conn.is_connected() {
                    if let Err(e) = conn.send_audio(&samples, timestamp).await {
                        warn!("Failed to send audio: {}", e);
                    } else {
                        packet_count += 1;
                        if packet_count % 100 == 0 {
                            tracing::debug!("Sent {} audio packets", packet_count);
                        }
                    }
                }
            }
        });

        // Process received audio on main thread using select
        let mut received_count = 0u64;
        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    info!("Shutting down...");
                    break;
                }
                Some(samples) = rx_playback.recv() => {
                    audio_engine.enqueue_playback(&samples);
                    received_count += 1;
                    if received_count % 100 == 0 {
                        tracing::debug!("Received {} audio packets for playback", received_count);
                    }
                }
            }
        }

        send_task.abort();

        let stats = {
            let mut conn = connection_arc.lock().await;
            let stats = conn.stats();
            conn.disconnect();
            stats
        };

        audio_engine.stop_capture();
        audio_engine.stop_playback();

        println!("\nSession statistics:");
        println!("  Packets sent: {}", stats.packets_sent);
        println!("  Packets received: {}", stats.packets_received);
        println!("  Bytes sent: {}", stats.bytes_sent);
        println!("  Bytes received: {}", stats.bytes_received);
    } else {
        println!("\nNo peers with addresses found in room. Waiting for peers...");
        println!("Press Ctrl+C to exit.\n");
        tokio::signal::ctrl_c().await?;
    }

    // Leave room
    let _ = conn.send(SignalingMessage::LeaveRoom).await;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    setup_logging(cli.verbose);

    match cli.command {
        Commands::Devices { action } => match action {
            DevicesAction::List => list_devices(),
        },
        Commands::Host {
            port,
            sample_rate,
            frame_size,
        } => {
            run_host(port, sample_rate, frame_size).await?;
        }
        Commands::Join {
            address,
            sample_rate,
            frame_size,
        } => {
            run_join(address, sample_rate, frame_size).await?;
        }
        Commands::Rooms { server } => {
            run_rooms(server).await?;
        }
        Commands::JoinRoom {
            server,
            room,
            name,
            sample_rate,
            frame_size,
        } => {
            run_join_room(server, room, name, sample_rate, frame_size).await?;
        }
    }

    Ok(())
}
