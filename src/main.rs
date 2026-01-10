//! jamjam - Low-latency P2P audio communication for musicians

use std::sync::Arc;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use jamjam::audio::{list_input_devices, list_output_devices, AudioConfig, AudioEngine};
use jamjam::network::Connection;

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

    // Connect to remote
    connection.connect(remote_addr).await?;

    // Set up audio receive callback - sends to channel instead of directly to engine
    connection.set_audio_callback(move |data, _timestamp| {
        // Convert bytes back to f32 samples
        let samples: Vec<f32> = data
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();

        let _ = tx_playback.try_send(samples);
    });

    println!("\nConnected to {}. Session active.", address);
    println!("Press Ctrl+C to stop.\n");

    // Spawn task to send captured audio
    let connection_arc = Arc::new(tokio::sync::Mutex::new(connection));
    let connection_for_send = connection_arc.clone();

    let send_task = tokio::spawn(async move {
        while let Some((samples, timestamp)) = rx_capture.recv().await {
            let conn = connection_for_send.lock().await;
            if conn.is_connected() {
                if let Err(e) = conn.send_audio(&samples, timestamp).await {
                    tracing::warn!("Failed to send audio: {}", e);
                }
            }
        }
    });

    // Spawn task to enqueue received audio for playback
    // Note: We need to process this in a way that doesn't require AudioEngine to be Send
    // For now, we'll just log received samples (playback handled via ring buffer in engine)
    let playback_task = tokio::spawn(async move {
        while let Some(samples) = rx_playback.recv().await {
            // Samples are enqueued via the ring buffer in the engine
            // This task just ensures the channel is drained
            tracing::debug!("Received {} samples for playback", samples.len());
        }
    });

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;

    info!("Shutting down...");
    send_task.abort();
    playback_task.abort();

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
    }

    Ok(())
}
