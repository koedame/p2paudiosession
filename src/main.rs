//! jamjam - Low-latency P2P audio communication for musicians

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

use jamjam::audio::{list_input_devices, list_output_devices, AudioConfig, AudioEngine, DeviceId};
use jamjam::network::{
    candidates_to_addrs, gather_candidates, Connection, ConnectionStats, LatencyBreakdown,
    LocalLatencyInfo, PeerLatencyInfo, SignalingClient, SignalingConnection, SignalingMessage,
};

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

        /// Input device name (use 'devices list' to see available devices)
        #[arg(long)]
        input_device: Option<String>,

        /// Output device name (use 'devices list' to see available devices)
        #[arg(long)]
        output_device: Option<String>,
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

        /// Input device name (use 'devices list' to see available devices)
        #[arg(long)]
        input_device: Option<String>,

        /// Output device name (use 'devices list' to see available devices)
        #[arg(long)]
        output_device: Option<String>,
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

        /// Input device name (use 'devices list' to see available devices)
        #[arg(long)]
        input_device: Option<String>,

        /// Output device name (use 'devices list' to see available devices)
        #[arg(long)]
        output_device: Option<String>,

        /// Send a single message and exit (non-interactive mode)
        #[arg(short = 'm', long)]
        message: Option<String>,

        /// Timeout in seconds for non-interactive mode (default: 5)
        #[arg(long, default_value = "5")]
        timeout: u64,

        /// Skip audio (chat only mode)
        #[arg(long)]
        chat_only: bool,
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
    match list_input_devices() {
        Ok(devices) => {
            for device in devices {
                let default_marker = if device.is_default { " (default)" } else { "" };
                println!("  - {}{}", device.name, default_marker);
            }
        }
        Err(e) => {
            println!("  Error: {}", e);
        }
    }

    println!("\nOutput devices:");
    match list_output_devices() {
        Ok(devices) => {
            for device in devices {
                let default_marker = if device.is_default { " (default)" } else { "" };
                println!("  - {}{}", device.name, default_marker);
            }
        }
        Err(e) => {
            println!("  Error: {}", e);
        }
    }
}

/// Print session statistics with latency breakdown
fn print_session_stats(
    stats: &ConnectionStats,
    local_info: &LocalLatencyInfo,
    peer_info: Option<&PeerLatencyInfo>,
    peer_name: Option<&str>,
) {
    let breakdown =
        LatencyBreakdown::calculate(local_info, peer_info, stats.rtt_ms, stats.jitter_ms);
    let peer_label = peer_name.unwrap_or("Peer");

    println!("\n═══════════════════════════════════════════════════════════════");
    println!(" Session Statistics");
    println!("═══════════════════════════════════════════════════════════════");

    // Network stats
    println!("\n Network:");
    println!("   RTT:          {:>7.2} ms", stats.rtt_ms);
    println!("   Jitter:       {:>7.2} ms", stats.jitter_ms);
    println!("   Packet Loss:  {:>7.1} %", stats.packet_loss_rate * 100.0);
    println!("   Uptime:       {:>7} sec", stats.uptime_seconds);

    // Latency breakdown
    println!("\n Latency Breakdown:");

    // Upstream (You -> Peer)
    println!("\n   Upstream (You → {}):", peer_label);
    println!(
        "     Capture buffer:    {:>6.2} ms  ({} samples @ {} Hz)",
        breakdown.upstream.capture_buffer_ms, local_info.frame_size, local_info.sample_rate
    );
    println!(
        "     Encode ({}):    {:>6.2} ms",
        local_info.codec, breakdown.upstream.encode_ms
    );
    println!(
        "     Network:           {:>6.2} ms  (RTT/2)",
        breakdown.upstream.network_ms
    );

    if breakdown.has_peer_info() {
        println!(
            "     [{}] Jitter buf: {:>6.2} ms",
            peer_label, breakdown.upstream.peer_jitter_buffer_ms
        );
        println!(
            "     [{}] Decode:     {:>6.2} ms",
            peer_label, breakdown.upstream.peer_decode_ms
        );
        println!(
            "     [{}] Playback:   {:>6.2} ms",
            peer_label, breakdown.upstream.peer_playback_buffer_ms
        );
    } else {
        println!("     [{}] (info not available)", peer_label);
    }
    println!("     ─────────────────────────────");
    println!(
        "     Total:             {:>6.2} ms",
        breakdown.upstream_total_ms
    );

    // Downstream (Peer -> You)
    println!("\n   Downstream ({} → You):", peer_label);
    if breakdown.has_peer_info() {
        println!(
            "     [{}] Capture:    {:>6.2} ms",
            peer_label, breakdown.downstream.peer_capture_buffer_ms
        );
        println!(
            "     [{}] Encode:     {:>6.2} ms",
            peer_label, breakdown.downstream.peer_encode_ms
        );
    } else {
        println!("     [{}] (info not available)", peer_label);
    }
    println!(
        "     Network:           {:>6.2} ms  (RTT/2)",
        breakdown.downstream.network_ms
    );
    println!(
        "     Jitter buffer:     {:>6.2} ms",
        breakdown.downstream.jitter_buffer_ms
    );
    println!(
        "     Decode ({}):    {:>6.2} ms",
        local_info.codec, breakdown.downstream.decode_ms
    );
    println!(
        "     Playback buffer:   {:>6.2} ms  ({} samples)",
        breakdown.downstream.playback_buffer_ms, local_info.frame_size
    );
    println!("     ─────────────────────────────");
    println!(
        "     Total:             {:>6.2} ms",
        breakdown.downstream_total_ms
    );

    // Summary
    println!("\n Summary:");
    println!(
        "   Upstream total:    {:>7.2} ms",
        breakdown.upstream_total_ms
    );
    println!(
        "   Downstream total:  {:>7.2} ms",
        breakdown.downstream_total_ms
    );
    println!(
        "   Round-trip total:  {:>7.2} ms",
        breakdown.roundtrip_total_ms
    );

    // Packet stats
    println!("\n Packets:");
    println!("   Sent:     {:>10}", stats.packets_sent);
    println!("   Received: {:>10}", stats.packets_received);
    println!("   Bytes sent:     {:>10}", stats.bytes_sent);
    println!("   Bytes received: {:>10}", stats.bytes_received);

    println!("\n═══════════════════════════════════════════════════════════════\n");
}

async fn run_host(
    port: u16,
    sample_rate: u32,
    frame_size: u32,
    input_device: Option<String>,
    output_device: Option<String>,
) -> Result<()> {
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

    let input_id = input_device.map(DeviceId);
    let output_id = output_device.map(DeviceId);

    // Start capture (for now just log, full implementation would track connected peers)
    audio_engine.start_capture(input_id.as_ref(), move |samples, timestamp| {
        if timestamp % 48000 == 0 {
            tracing::debug!(
                "Captured {} samples at timestamp {}",
                samples.len(),
                timestamp
            );
        }
    })?;

    audio_engine.start_playback(output_id.as_ref())?;

    println!("\nHost started. Listening on port {}.", port);
    println!("Press Ctrl+C to stop.\n");

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;

    info!("Shutting down...");
    audio_engine.stop_capture();
    audio_engine.stop_playback();

    let stats = connection.stats();
    let peer_info = connection.peer_latency_info();
    let local_info = LocalLatencyInfo::from_audio_config(frame_size, sample_rate, "pcm");

    print_session_stats(&stats, &local_info, peer_info.as_ref(), None);

    Ok(())
}

async fn run_join(
    address: String,
    sample_rate: u32,
    frame_size: u32,
    input_device: Option<String>,
    output_device: Option<String>,
) -> Result<()> {
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

    let input_id = input_device.map(DeviceId);
    let output_id = output_device.map(DeviceId);

    // Create channels for audio data
    // tx_capture: capture callback -> send task
    let (tx_capture, mut rx_capture) = tokio::sync::mpsc::channel::<(Vec<f32>, u32)>(64);
    // tx_playback: receive callback -> playback task
    let (tx_playback, mut rx_playback) = tokio::sync::mpsc::channel::<Vec<f32>>(64);

    // Start audio capture
    audio_engine.start_capture(input_id.as_ref(), move |samples, timestamp| {
        let _ = tx_capture.try_send((samples.to_vec(), timestamp as u32));
    })?;

    audio_engine.start_playback(output_id.as_ref())?;

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
                    if packet_count.is_multiple_of(100) {
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
                if received_count.is_multiple_of(100) {
                    tracing::debug!("Received {} audio packets for playback", received_count);
                }
            }
        }
    }

    send_task.abort();

    let (stats, peer_info) = {
        let mut conn = connection_arc.lock().await;
        let stats = conn.stats();
        let peer_info = conn.peer_latency_info();
        conn.disconnect();
        (stats, peer_info)
    };

    audio_engine.stop_capture();
    audio_engine.stop_playback();

    let local_info = LocalLatencyInfo::from_audio_config(frame_size, sample_rate, "pcm");
    print_session_stats(&stats, &local_info, peer_info.as_ref(), None);

    Ok(())
}

/// Process signaling events and print them to stdout
fn handle_signaling_event(msg: &SignalingMessage) {
    match msg {
        SignalingMessage::PeerJoined { peer } => {
            println!("\n📥 {} joined the room", peer.name);
            print!("chat> ");
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }
        SignalingMessage::PeerLeft { peer_id } => {
            println!("\n📤 Peer {} left the room", peer_id);
            print!("chat> ");
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }
        SignalingMessage::ChatMessage {
            sender_name,
            content,
            ..
        } => {
            println!("\n💬 {}: {}", sender_name, content);
            print!("chat> ");
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }
        _ => {}
    }
}

/// Send a chat message via signaling connection
async fn send_chat_message(
    conn: &mut SignalingConnection,
    peer_id: &str,
    peer_name: &str,
    content: &str,
) -> Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    conn.send(SignalingMessage::ChatMessage {
        sender_id: peer_id.to_string(),
        sender_name: peer_name.to_string(),
        content: content.to_string(),
        timestamp,
    })
    .await?;

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
                    let password_str = if room.has_password {
                        " (password protected)"
                    } else {
                        ""
                    };
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

#[allow(clippy::too_many_arguments)]
async fn run_join_room(
    server: String,
    room_id: String,
    peer_name: String,
    sample_rate: u32,
    frame_size: u32,
    input_device: Option<String>,
    output_device: Option<String>,
    message: Option<String>,
    timeout_secs: u64,
    chat_only: bool,
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

    let (my_peer_id, peers) = match conn.recv().await? {
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
            (peer_id, peers)
        }
        SignalingMessage::Error { message } => {
            anyhow::bail!("Failed to join room: {}", message);
        }
        _ => {
            anyhow::bail!("Unexpected response from server");
        }
    };

    // Find a peer with address(es) to connect to
    // Prefer peers with candidates, fall back to legacy public_addr
    // Skip peer search if chat_only mode is enabled
    let target_peer = if chat_only {
        None
    } else {
        peers
            .iter()
            .find(|p| !p.candidates.is_empty() || p.public_addr.is_some())
    };

    if let Some(peer) = target_peer {
        let peer_display_name = peer.name.clone();

        // Collect remote addresses to try (prefer candidates, fall back to legacy)
        let remote_addrs: Vec<std::net::SocketAddr> = if !peer.candidates.is_empty() {
            candidates_to_addrs(&peer.candidates)
        } else if let Some(addr) = peer.public_addr {
            vec![addr]
        } else {
            vec![]
        };

        println!(
            "\nConnecting to peer {} ({} address candidates)...",
            peer.name,
            remote_addrs.len()
        );
        for (i, addr) in remote_addrs.iter().enumerate() {
            info!("  Candidate {}: {}", i, addr);
        }

        let mut connection = Connection::new("0.0.0.0:0").await?;
        let local_addr = connection.local_addr();
        info!("Local UDP socket: {}", local_addr);

        // Gather our local candidates
        println!("Gathering local address candidates...");
        let candidates = gather_candidates(local_addr.port()).await;
        info!("Gathered {} candidates", candidates.len());
        for c in &candidates {
            info!("  {:?}: {}", c.candidate_type, c.address);
        }

        // Update our peer info with candidates
        conn.send(SignalingMessage::UpdatePeerInfo {
            candidates: candidates.clone(),
            public_addr: candidates.first().map(|c| c.address),
            local_addr: Some(local_addr),
        })
        .await?;

        let mut audio_engine = AudioEngine::new(config.clone());

        let input_id = input_device.map(DeviceId);
        let output_id = output_device.map(DeviceId);

        // Create channels for audio data
        let (tx_capture, mut rx_capture) = tokio::sync::mpsc::channel::<(Vec<f32>, u32)>(64);
        let (tx_playback, mut rx_playback) = tokio::sync::mpsc::channel::<Vec<f32>>(64);

        // Start audio capture
        audio_engine.start_capture(input_id.as_ref(), move |samples, timestamp| {
            let _ = tx_capture.try_send((samples.to_vec(), timestamp as u32));
        })?;

        audio_engine.start_playback(output_id.as_ref())?;

        // Set up audio receive callback BEFORE connect
        // (connect starts receive loop which clones the callback)
        connection.set_audio_callback(move |data, _timestamp| {
            let samples: Vec<f32> = data
                .chunks_exact(4)
                .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect();
            let _ = tx_playback.try_send(samples);
        });

        // Connect to remote peer using candidates (Happy Eyeballs style)
        connection.connect_with_candidates(&remote_addrs).await?;

        println!("\nConnected to peer. Session active.");
        println!("Audio config: {:?}", config);
        println!("\n💬 Chat enabled. Type a message and press Enter to send.");
        println!("Press Ctrl+C to stop.\n");
        print!("chat> ");
        let _ = std::io::Write::flush(&mut std::io::stdout());

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
                        if packet_count.is_multiple_of(100) {
                            tracing::debug!("Sent {} audio packets", packet_count);
                        }
                    }
                }
            }
        });

        // Wrap signaling connection in Arc<Mutex> for sharing
        let signaling_conn_arc = Arc::new(tokio::sync::Mutex::new(conn));
        let signaling_for_recv = signaling_conn_arc.clone();

        // Channel for signaling events
        let (tx_signaling, mut rx_signaling) = tokio::sync::mpsc::channel::<SignalingMessage>(32);

        // Spawn task to poll signaling events
        let signaling_recv_task = tokio::spawn(async move {
            loop {
                let msg = {
                    let mut conn_guard = signaling_for_recv.lock().await;
                    // Use timeout to avoid blocking forever
                    match tokio::time::timeout(Duration::from_millis(100), conn_guard.recv()).await
                    {
                        Ok(Ok(msg)) => Some(msg),
                        Ok(Err(_)) => None, // Connection error
                        Err(_) => None,     // Timeout
                    }
                };

                if let Some(msg) = msg {
                    if tx_signaling.send(msg).await.is_err() {
                        break;
                    }
                }

                // Small delay to avoid busy loop
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        });

        // Setup stdin reader
        let stdin = tokio::io::stdin();
        let mut stdin_reader = BufReader::new(stdin).lines();

        // Store peer info for chat
        let my_peer_id_for_chat = my_peer_id.to_string();
        let peer_name_for_chat = peer_name.clone();

        // Process received audio and chat on main thread using select
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
                    if received_count.is_multiple_of(100) {
                        tracing::debug!("Received {} audio packets for playback", received_count);
                    }
                }
                Some(msg) = rx_signaling.recv() => {
                    // Skip our own chat messages to avoid duplicate display
                    if let SignalingMessage::ChatMessage { sender_id, .. } = &msg {
                        if sender_id == &my_peer_id_for_chat {
                            continue;
                        }
                    }
                    handle_signaling_event(&msg);
                }
                line_result = stdin_reader.next_line() => {
                    match line_result {
                        Ok(Some(line)) => {
                            let line = line.trim();
                            if !line.is_empty() {
                                let mut conn_guard = signaling_conn_arc.lock().await;
                                if let Err(e) = send_chat_message(
                                    &mut conn_guard,
                                    &my_peer_id_for_chat,
                                    &peer_name_for_chat,
                                    line,
                                ).await {
                                    warn!("Failed to send chat: {}", e);
                                } else {
                                    println!("💬 You: {}", line);
                                }
                            }
                            print!("chat> ");
                            let _ = std::io::Write::flush(&mut std::io::stdout());
                        }
                        Ok(None) => {
                            // EOF
                            info!("stdin closed");
                        }
                        Err(e) => {
                            warn!("stdin error: {}", e);
                        }
                    }
                }
            }
        }

        send_task.abort();
        signaling_recv_task.abort();

        let (stats, peer_latency_info) = {
            let mut conn = connection_arc.lock().await;
            let stats = conn.stats();
            let peer_latency_info = conn.peer_latency_info();
            conn.disconnect();
            (stats, peer_latency_info)
        };

        audio_engine.stop_capture();
        audio_engine.stop_playback();

        let local_info = LocalLatencyInfo::from_audio_config(frame_size, sample_rate, "pcm");
        print_session_stats(
            &stats,
            &local_info,
            peer_latency_info.as_ref(),
            Some(&peer_display_name),
        );

        // Leave room
        {
            let mut conn_guard = signaling_conn_arc.lock().await;
            let _ = conn_guard.send(SignalingMessage::LeaveRoom).await;
        }

        return Ok(());
    } else {
        // Chat-only mode (no audio connection)
        if chat_only {
            println!("\n📝 Chat-only mode (no audio connection)");
        } else {
            println!("\nNo peers with addresses found in room. Waiting for peers...");
        }

        // Channel for signaling events
        let (tx_signaling, mut rx_signaling) = tokio::sync::mpsc::channel::<SignalingMessage>(32);

        // Wrap signaling connection
        let signaling_conn_arc = Arc::new(tokio::sync::Mutex::new(conn));
        let signaling_for_recv = signaling_conn_arc.clone();

        // Spawn task to poll signaling events
        let signaling_recv_task = tokio::spawn(async move {
            loop {
                let msg = {
                    let mut conn_guard = signaling_for_recv.lock().await;
                    match tokio::time::timeout(Duration::from_millis(100), conn_guard.recv()).await
                    {
                        Ok(Ok(msg)) => Some(msg),
                        Ok(Err(_)) => None,
                        Err(_) => None,
                    }
                };

                if let Some(msg) = msg {
                    if tx_signaling.send(msg).await.is_err() {
                        break;
                    }
                }

                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        });

        let my_peer_id_for_chat = my_peer_id.to_string();
        let peer_name_for_chat = peer_name.clone();

        // Non-interactive mode: send message and wait for response
        if let Some(msg_to_send) = message {
            println!("📤 Sending: {}", msg_to_send);

            {
                let mut conn_guard = signaling_conn_arc.lock().await;
                send_chat_message(
                    &mut conn_guard,
                    &my_peer_id_for_chat,
                    &peer_name_for_chat,
                    &msg_to_send,
                )
                .await?;
            }

            // Wait for response with timeout
            let timeout_duration = Duration::from_secs(timeout_secs);
            let start = std::time::Instant::now();
            let mut received_response = false;

            println!("⏳ Waiting for response (timeout: {}s)...", timeout_secs);

            loop {
                if start.elapsed() > timeout_duration {
                    if !received_response {
                        println!("⚠️  Timeout: no response received");
                    }
                    break;
                }

                tokio::select! {
                    Some(msg) = rx_signaling.recv() => {
                        if let SignalingMessage::ChatMessage { sender_id, sender_name, content, .. } = &msg {
                            // Skip our own messages to avoid duplicate display
                            if sender_id != &my_peer_id_for_chat {
                                println!("📥 {}: {}", sender_name, content);
                                received_response = true;
                                // Give a small window for additional messages
                                tokio::time::sleep(Duration::from_millis(500)).await;
                            }
                        } else {
                            handle_signaling_event(&msg);
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(100)) => {}
                }
            }

            signaling_recv_task.abort();

            // Leave room
            {
                let mut conn_guard = signaling_conn_arc.lock().await;
                let _ = conn_guard.send(SignalingMessage::LeaveRoom).await;
            }

            return Ok(());
        }

        // Interactive mode
        println!("\n💬 Chat enabled. Type a message and press Enter to send.");
        println!("Press Ctrl+C to exit.\n");
        print!("chat> ");
        let _ = std::io::Write::flush(&mut std::io::stdout());

        // Setup stdin reader
        let stdin = tokio::io::stdin();
        let mut stdin_reader = BufReader::new(stdin).lines();

        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    info!("Shutting down...");
                    break;
                }
                Some(msg) = rx_signaling.recv() => {
                    // Skip our own chat messages to avoid duplicate display
                    if let SignalingMessage::ChatMessage { sender_id, .. } = &msg {
                        if sender_id == &my_peer_id_for_chat {
                            continue;
                        }
                    }
                    handle_signaling_event(&msg);
                }
                line_result = stdin_reader.next_line() => {
                    match line_result {
                        Ok(Some(line)) => {
                            let line = line.trim();
                            if !line.is_empty() {
                                let mut conn_guard = signaling_conn_arc.lock().await;
                                if let Err(e) = send_chat_message(
                                    &mut conn_guard,
                                    &my_peer_id_for_chat,
                                    &peer_name_for_chat,
                                    line,
                                ).await {
                                    warn!("Failed to send chat: {}", e);
                                } else {
                                    println!("💬 You: {}", line);
                                }
                            }
                            print!("chat> ");
                            let _ = std::io::Write::flush(&mut std::io::stdout());
                        }
                        Ok(None) => {
                            info!("stdin closed");
                        }
                        Err(e) => {
                            warn!("stdin error: {}", e);
                        }
                    }
                }
            }
        }

        signaling_recv_task.abort();

        // Leave room
        {
            let mut conn_guard = signaling_conn_arc.lock().await;
            let _ = conn_guard.send(SignalingMessage::LeaveRoom).await;
        }
    }

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
            input_device,
            output_device,
        } => {
            run_host(port, sample_rate, frame_size, input_device, output_device).await?;
        }
        Commands::Join {
            address,
            sample_rate,
            frame_size,
            input_device,
            output_device,
        } => {
            run_join(
                address,
                sample_rate,
                frame_size,
                input_device,
                output_device,
            )
            .await?;
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
            input_device,
            output_device,
            message,
            timeout,
            chat_only,
        } => {
            run_join_room(
                server,
                room,
                name,
                sample_rate,
                frame_size,
                input_device,
                output_device,
                message,
                timeout,
                chat_only,
            )
            .await?;
        }
    }

    Ok(())
}
