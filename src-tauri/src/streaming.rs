//! Audio streaming IPC commands for Tauri
//!
//! Manages P2P audio streaming with a dedicated audio thread to handle
//! the non-Send+Sync AudioEngine.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, RwLock};
use std::thread;

use serde::Serialize;
use tokio::sync::Mutex;

use jamjam::audio::{AudioConfig, AudioEngine, DeviceId};
use jamjam::network::{Connection, ConnectionStats, LatencyBreakdown, LocalLatencyInfo};

/// Audio configuration used for latency calculations
const AUDIO_SAMPLE_RATE: u32 = 48000;
const AUDIO_FRAME_SIZE: u32 = 128;

/// Streaming state managed by Tauri
pub struct StreamingState {
    /// Command sender to audio thread (None if not started)
    cmd_tx: Mutex<Option<std_mpsc::Sender<StreamingCommand>>>,
    /// Flag indicating if streaming is active
    is_active: Arc<AtomicBool>,
    /// Remote address currently connected to
    remote_addr: Mutex<Option<String>>,
    /// Connection statistics (updated by audio thread)
    stats: Arc<RwLock<Option<ConnectionStats>>>,
}

impl StreamingState {
    pub fn new() -> Self {
        Self {
            cmd_tx: Mutex::new(None),
            is_active: Arc::new(AtomicBool::new(false)),
            remote_addr: Mutex::new(None),
            stats: Arc::new(RwLock::new(None)),
        }
    }

    /// Get local latency info based on audio config
    fn local_latency_info(&self) -> LocalLatencyInfo {
        LocalLatencyInfo::from_audio_config(AUDIO_FRAME_SIZE, AUDIO_SAMPLE_RATE, "pcm")
    }
}

impl Default for StreamingState {
    fn default() -> Self {
        Self::new()
    }
}

/// Commands sent to the audio thread
enum StreamingCommand {
    Stop,
    SetInputDevice(Option<String>),
    SetOutputDevice(Option<String>),
}

/// Network statistics for IPC
#[derive(Debug, Clone, Serialize)]
pub struct NetworkStats {
    /// Round-trip time in milliseconds
    pub rtt_ms: f32,
    /// Jitter in milliseconds
    pub jitter_ms: f32,
    /// Packet loss percentage (0-100)
    pub packet_loss_percent: f32,
    /// Connection uptime in seconds
    pub uptime_seconds: u64,
    /// Total packets sent
    pub packets_sent: u64,
    /// Total packets received
    pub packets_received: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
}

/// Latency component breakdown for IPC
#[derive(Debug, Clone, Serialize)]
pub struct LatencyComponent {
    /// Component name
    pub name: String,
    /// Latency in milliseconds
    pub ms: f32,
    /// Additional info (e.g., "128 samples @ 48000 Hz")
    pub info: Option<String>,
}

/// Detailed latency breakdown for IPC
#[derive(Debug, Clone, Serialize)]
pub struct DetailedLatency {
    /// Upstream components (self -> peer)
    pub upstream: Vec<LatencyComponent>,
    /// Upstream total in ms
    pub upstream_total_ms: f32,
    /// Downstream components (peer -> self)
    pub downstream: Vec<LatencyComponent>,
    /// Downstream total in ms
    pub downstream_total_ms: f32,
    /// Round-trip total in ms
    pub roundtrip_total_ms: f32,
}

/// Streaming status for IPC
#[derive(Debug, Clone, Serialize)]
pub struct StreamingStatus {
    pub is_active: bool,
    pub remote_addr: Option<String>,
    /// Network statistics
    pub network: Option<NetworkStats>,
    /// Detailed latency breakdown
    pub latency: Option<DetailedLatency>,
}

/// Start audio streaming to a remote peer
#[tauri::command]
pub async fn streaming_start(
    remote_addr: String,
    input_device_id: Option<String>,
    output_device_id: Option<String>,
    state: tauri::State<'_, StreamingState>,
) -> Result<(), String> {
    // Check if already streaming
    if state.is_active.load(Ordering::SeqCst) {
        return Err("Streaming already active".to_string());
    }

    // Parse remote address
    let addr: SocketAddr = remote_addr
        .parse()
        .map_err(|e| format!("Invalid address: {}", e))?;

    // Create channel for commands to audio thread
    let (cmd_tx, cmd_rx) = std_mpsc::channel::<StreamingCommand>();

    // Store command sender
    {
        let mut tx = state.cmd_tx.lock().await;
        *tx = Some(cmd_tx);
    }

    // Store remote address
    {
        let mut addr_lock = state.remote_addr.lock().await;
        *addr_lock = Some(remote_addr.clone());
    }

    let is_active = state.is_active.clone();
    let shared_stats = state.stats.clone();

    // Mark as active BEFORE spawning thread to avoid race condition
    state.is_active.store(true, Ordering::SeqCst);

    // Spawn audio thread
    thread::spawn(move || {
        // Create tokio runtime for this thread
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");

        rt.block_on(async move {
            if let Err(e) = run_audio_streaming(
                addr,
                input_device_id,
                output_device_id,
                cmd_rx,
                &is_active,
                &shared_stats,
            )
            .await
            {
                eprintln!("Audio streaming error: {}", e);
            }
            is_active.store(false, Ordering::SeqCst);
            // Clear stats on disconnect
            if let Ok(mut stats) = shared_stats.write() {
                *stats = None;
            }
        });
    });

    Ok(())
}

/// Stop audio streaming
#[tauri::command]
pub async fn streaming_stop(state: tauri::State<'_, StreamingState>) -> Result<(), String> {
    if !state.is_active.load(Ordering::SeqCst) {
        return Ok(()); // Already stopped
    }

    // Send stop command
    {
        let tx = state.cmd_tx.lock().await;
        if let Some(ref sender) = *tx {
            let _ = sender.send(StreamingCommand::Stop);
        }
    }

    // Clear state
    {
        let mut tx = state.cmd_tx.lock().await;
        *tx = None;
    }
    {
        let mut addr = state.remote_addr.lock().await;
        *addr = None;
    }
    {
        if let Ok(mut stats) = state.stats.write() {
            *stats = None;
        }
    }

    // Wait briefly for thread to finish
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    state.is_active.store(false, Ordering::SeqCst);

    Ok(())
}

/// Get streaming status
#[tauri::command]
pub async fn streaming_status(
    state: tauri::State<'_, StreamingState>,
) -> Result<StreamingStatus, String> {
    let is_active = state.is_active.load(Ordering::SeqCst);
    let remote_addr = state.remote_addr.lock().await.clone();
    let stats = state.stats.read().ok().and_then(|s| s.clone());

    let (network, latency) = if let Some(ref s) = stats {
        let local_info = state.local_latency_info();
        let breakdown = LatencyBreakdown::calculate(&local_info, None, s.rtt_ms, s.jitter_ms);

        let network = NetworkStats {
            rtt_ms: s.rtt_ms,
            jitter_ms: s.jitter_ms,
            packet_loss_percent: s.packet_loss_rate * 100.0,
            uptime_seconds: s.uptime_seconds,
            packets_sent: s.packets_sent,
            packets_received: s.packets_received,
            bytes_sent: s.bytes_sent,
            bytes_received: s.bytes_received,
        };

        let upstream = vec![
            LatencyComponent {
                name: "Capture buffer".to_string(),
                ms: breakdown.upstream.capture_buffer_ms,
                info: Some(format!(
                    "{} samples @ {} Hz",
                    AUDIO_FRAME_SIZE, AUDIO_SAMPLE_RATE
                )),
            },
            LatencyComponent {
                name: "Encode (pcm)".to_string(),
                ms: breakdown.upstream.encode_ms,
                info: None,
            },
            LatencyComponent {
                name: "Network".to_string(),
                ms: breakdown.upstream.network_ms,
                info: Some("RTT/2".to_string()),
            },
        ];

        let downstream = vec![
            LatencyComponent {
                name: "Network".to_string(),
                ms: breakdown.downstream.network_ms,
                info: Some("RTT/2".to_string()),
            },
            LatencyComponent {
                name: "Jitter buffer".to_string(),
                ms: breakdown.downstream.jitter_buffer_ms,
                info: None,
            },
            LatencyComponent {
                name: "Decode (pcm)".to_string(),
                ms: breakdown.downstream.decode_ms,
                info: None,
            },
            LatencyComponent {
                name: "Playback buffer".to_string(),
                ms: breakdown.downstream.playback_buffer_ms,
                info: Some(format!("{} samples", AUDIO_FRAME_SIZE)),
            },
        ];

        let latency = DetailedLatency {
            upstream,
            upstream_total_ms: breakdown.upstream_total_ms,
            downstream,
            downstream_total_ms: breakdown.downstream_total_ms,
            roundtrip_total_ms: breakdown.roundtrip_total_ms,
        };

        (Some(network), Some(latency))
    } else {
        (None, None)
    };

    Ok(StreamingStatus {
        is_active,
        remote_addr,
        network,
        latency,
    })
}

/// Set input device during streaming
#[tauri::command]
pub async fn streaming_set_input_device(
    device_id: Option<String>,
    state: tauri::State<'_, StreamingState>,
) -> Result<(), String> {
    if !state.is_active.load(Ordering::SeqCst) {
        return Err("Streaming is not active".to_string());
    }

    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        sender
            .send(StreamingCommand::SetInputDevice(device_id))
            .map_err(|e| format!("Failed to send command: {}", e))?;
    }

    Ok(())
}

/// Set output device during streaming
#[tauri::command]
pub async fn streaming_set_output_device(
    device_id: Option<String>,
    state: tauri::State<'_, StreamingState>,
) -> Result<(), String> {
    if !state.is_active.load(Ordering::SeqCst) {
        return Err("Streaming is not active".to_string());
    }

    let tx = state.cmd_tx.lock().await;
    if let Some(ref sender) = *tx {
        sender
            .send(StreamingCommand::SetOutputDevice(device_id))
            .map_err(|e| format!("Failed to send command: {}", e))?;
    }

    Ok(())
}

/// Run audio streaming in the audio thread
async fn run_audio_streaming(
    remote_addr: SocketAddr,
    input_device_id: Option<String>,
    output_device_id: Option<String>,
    cmd_rx: std_mpsc::Receiver<StreamingCommand>,
    is_active: &AtomicBool,
    shared_stats: &RwLock<Option<ConnectionStats>>,
) -> Result<(), String> {
    // Audio configuration
    let config = AudioConfig {
        sample_rate: 48000,
        channels: 1,
        frame_size: 128,
    };

    // Create connection
    let mut connection = Connection::new("0.0.0.0:0")
        .await
        .map_err(|e| format!("Failed to create connection: {}", e))?;

    // Create audio engine
    let mut audio_engine = AudioEngine::new(config);

    let input_id = input_device_id.map(DeviceId);
    let output_id = output_device_id.map(DeviceId);

    // Create channels for audio data
    let (tx_capture, mut rx_capture) = tokio::sync::mpsc::channel::<(Vec<f32>, u32)>(64);
    let (tx_playback, mut rx_playback) = tokio::sync::mpsc::channel::<Vec<f32>>(64);

    // Keep a clone for device switching
    let tx_capture_for_switch = tx_capture.clone();

    // Start audio capture
    audio_engine
        .start_capture(input_id.as_ref(), move |samples, timestamp| {
            let _ = tx_capture.try_send((samples.to_vec(), timestamp as u32));
        })
        .map_err(|e| format!("Failed to start capture: {}", e))?;

    // Start audio playback
    audio_engine
        .start_playback(output_id.as_ref())
        .map_err(|e| format!("Failed to start playback: {}", e))?;

    // Set up audio receive callback BEFORE connect
    connection.set_audio_callback(move |data, _timestamp| {
        // Convert bytes back to f32 samples
        let samples: Vec<f32> = data
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();

        let _ = tx_playback.try_send(samples);
    });

    // Connect to remote peer
    connection
        .connect(remote_addr)
        .await
        .map_err(|e| format!("Failed to connect: {}", e))?;

    println!("Connected to {}. Streaming active.", remote_addr);

    // Wrap connection for shared access
    let connection_arc = Arc::new(tokio::sync::Mutex::new(connection));
    let connection_for_send = connection_arc.clone();

    // Spawn task to send captured audio
    let send_task = tokio::spawn(async move {
        while let Some((samples, timestamp)) = rx_capture.recv().await {
            let conn = connection_for_send.lock().await;
            if conn.is_connected() {
                if let Err(e) = conn.send_audio(&samples, timestamp).await {
                    eprintln!("Failed to send audio: {}", e);
                }
            }
        }
    });

    // Main loop: process received audio and check for stop command
    let mut stats_update_counter = 0u32;
    loop {
        // Check for commands (non-blocking)
        match cmd_rx.try_recv() {
            Ok(StreamingCommand::Stop) => {
                println!("Stopping streaming...");
                break;
            }
            Ok(StreamingCommand::SetInputDevice(device_id)) => {
                println!("Switching input device to: {:?}", device_id);
                let new_device_id = device_id.map(DeviceId);
                let tx_clone = tx_capture_for_switch.clone();
                if let Err(e) = audio_engine.set_input_device(
                    new_device_id.as_ref(),
                    move |samples, timestamp| {
                        let _ = tx_clone.try_send((samples.to_vec(), timestamp as u32));
                    },
                ) {
                    eprintln!("Failed to switch input device: {}", e);
                }
            }
            Ok(StreamingCommand::SetOutputDevice(device_id)) => {
                println!("Switching output device to: {:?}", device_id);
                let new_device_id = device_id.map(DeviceId);
                if let Err(e) = audio_engine.set_output_device(new_device_id.as_ref()) {
                    eprintln!("Failed to switch output device: {}", e);
                }
            }
            Err(std_mpsc::TryRecvError::Disconnected) => {
                println!("Command channel disconnected");
                break;
            }
            Err(std_mpsc::TryRecvError::Empty) => {
                // No command, continue
            }
        }

        // Check if we should still be active
        if !is_active.load(Ordering::SeqCst) {
            break;
        }

        // Update stats every ~100ms (10 iterations * 10ms)
        stats_update_counter += 1;
        if stats_update_counter >= 10 {
            stats_update_counter = 0;
            if let Ok(conn) = connection_arc.try_lock() {
                let conn_stats = conn.stats();
                if let Ok(mut stats) = shared_stats.write() {
                    *stats = Some(conn_stats);
                }
            }
        }

        // Process received audio with timeout
        tokio::select! {
            Some(samples) = rx_playback.recv() => {
                audio_engine.enqueue_playback(&samples);
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(10)) => {
                // Timeout, check commands again
            }
        }
    }

    // Cleanup
    send_task.abort();

    {
        let mut conn = connection_arc.lock().await;
        conn.disconnect();
    }

    audio_engine.stop_capture();
    audio_engine.stop_playback();

    println!("Streaming stopped.");

    Ok(())
}
