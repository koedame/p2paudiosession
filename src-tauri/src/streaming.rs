//! Audio streaming IPC commands for Tauri
//!
//! Manages P2P audio streaming with a dedicated audio thread to handle
//! the non-Send+Sync AudioEngine.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, RwLock};
use std::thread;

use serde::Serialize;
use tokio::sync::Mutex;

use jamjam::audio::{AudioConfig, AudioEngine, DeviceId};
use jamjam::network::{Connection, ConnectionStats, LatencyBreakdown, LocalLatencyInfo};

/// Audio sample rate used for latency calculations
/// Target: < 2ms one-way app latency (see CLAUDE.md requirements)
const AUDIO_SAMPLE_RATE: u32 = 48000;

/// Real-time thread priority for Linux (1-99, higher = more priority)
/// 99 = maximum, but risks system freeze if thread hangs
/// 90 = very high, leaves headroom for critical kernel threads
#[cfg(target_os = "linux")]
const REALTIME_PRIORITY: i32 = 90;

/// Set real-time priority for the CURRENT thread (call from within audio thread)
/// This reduces buffer underruns by giving the audio thread scheduling priority
#[cfg(target_os = "macos")]
fn set_current_thread_realtime_priority() {
    // Use macOS Mach thread policy API - this is what professional DAWs use
    // (Logic Pro, Ableton, Pro Tools, etc.)
    // Does NOT require root, and is more effective than SCHED_FIFO on macOS

    #[repr(C)]
    struct ThreadTimeConstraintPolicy {
        period: u32,        // Interval between processing (in Mach absolute time units)
        computation: u32,   // Time needed for computation
        constraint: u32,    // Maximum time before deadline
        preemptible: i32,   // Can be preempted?
    }

    const THREAD_TIME_CONSTRAINT_POLICY: u32 = 2;
    const THREAD_TIME_CONSTRAINT_POLICY_COUNT: u32 = 4;

    extern "C" {
        fn mach_thread_self() -> u32;
        fn thread_policy_set(
            thread: u32,
            flavor: u32,
            policy_info: *const ThreadTimeConstraintPolicy,
            count: u32,
        ) -> i32;
        fn mach_timebase_info(info: *mut MachTimebaseInfo) -> i32;
    }

    #[repr(C)]
    struct MachTimebaseInfo {
        numer: u32,
        denom: u32,
    }

    unsafe {
        // Get timebase info to convert nanoseconds to Mach absolute time
        let mut timebase = MachTimebaseInfo { numer: 0, denom: 0 };
        mach_timebase_info(&mut timebase);

        // Convert nanoseconds to Mach absolute time units
        let ns_to_abs = |ns: u64| -> u32 {
            ((ns * timebase.denom as u64) / timebase.numer as u64) as u32
        };

        // Audio timing constraints (for 48kHz, ~1ms period with small buffer)
        // period: how often the thread runs (e.g., every 1ms for audio callback)
        // computation: how much CPU time it needs per period
        // constraint: deadline (must complete within this time)
        let period_ns = 1_000_000;       // 1ms period (audio callback interval)
        let computation_ns = 500_000;    // 0.5ms computation time
        let constraint_ns = 1_000_000;   // 1ms deadline

        let policy = ThreadTimeConstraintPolicy {
            period: ns_to_abs(period_ns),
            computation: ns_to_abs(computation_ns),
            constraint: ns_to_abs(constraint_ns),
            preemptible: 0,  // Don't preempt during computation
        };

        let thread = mach_thread_self();
        let result = thread_policy_set(
            thread,
            THREAD_TIME_CONSTRAINT_POLICY,
            &policy,
            THREAD_TIME_CONSTRAINT_POLICY_COUNT,
        );

        if result == 0 {
            println!("Audio thread: macOS real-time priority enabled (TIME_CONSTRAINT)");
        } else {
            // Fall back to nice value
            libc::setpriority(libc::PRIO_PROCESS, 0, -20);
            println!("Audio thread: using nice -20 (TIME_CONSTRAINT failed: {})", result);
        }
    }
}

#[cfg(target_os = "linux")]
fn set_current_thread_realtime_priority() {
    unsafe {
        let policy = libc::SCHED_FIFO;
        let mut param: libc::sched_param = std::mem::zeroed();
        param.sched_priority = REALTIME_PRIORITY;
        let result = libc::pthread_setschedparam(libc::pthread_self(), policy, &param);
        if result != 0 {
            // Fall back to nice value if SCHED_FIFO fails (requires rtprio limit)
            libc::setpriority(libc::PRIO_PROCESS, 0, -20);
            println!("Audio thread: using nice -20 (SCHED_FIFO requires rtprio config)");
        } else {
            println!("Audio thread: real-time priority enabled (SCHED_FIFO {})", REALTIME_PRIORITY);
        }
    }
}

#[cfg(target_os = "windows")]
fn set_current_thread_realtime_priority() {
    use windows::Win32::System::Threading::{
        GetCurrentProcess, GetCurrentThread, SetPriorityClass, SetThreadPriority,
        HIGH_PRIORITY_CLASS, THREAD_PRIORITY_TIME_CRITICAL,
    };

    unsafe {
        // First, elevate the process priority class
        let _ = SetPriorityClass(GetCurrentProcess(), HIGH_PRIORITY_CLASS);

        // Then set thread to TIME_CRITICAL (highest within the priority class)
        let result = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_TIME_CRITICAL);
        if result.is_ok() {
            println!("Audio thread: real-time priority enabled (HIGH + TIME_CRITICAL)");
        } else {
            println!("Audio thread: failed to set thread priority");
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn set_current_thread_realtime_priority() {
    println!("Audio thread: real-time priority not supported on this platform");
}

/// Streaming state managed by Tauri
pub struct StreamingState {
    /// Command sender to audio thread (None if not started)
    cmd_tx: Mutex<Option<std_mpsc::Sender<StreamingCommand>>>,
    /// Flag indicating if streaming is active
    is_active: Arc<AtomicBool>,
    /// Flag indicating if microphone is muted
    is_muted: Arc<AtomicBool>,
    /// Current input audio level (0-100, RMS normalized)
    input_level: Arc<AtomicU32>,
    /// Remote address currently connected to
    remote_addr: Mutex<Option<String>>,
    /// Connection statistics (updated by audio thread)
    stats: Arc<RwLock<Option<ConnectionStats>>>,
    /// Current buffer size (frame_size) for latency calculations
    buffer_size: Mutex<u32>,
    /// Buffer underrun count (audio glitches due to CPU/scheduling)
    underrun_count: Arc<AtomicU64>,
}

impl StreamingState {
    pub fn new() -> Self {
        Self {
            cmd_tx: Mutex::new(None),
            is_active: Arc::new(AtomicBool::new(false)),
            is_muted: Arc::new(AtomicBool::new(false)),
            input_level: Arc::new(AtomicU32::new(0)),
            remote_addr: Mutex::new(None),
            stats: Arc::new(RwLock::new(None)),
            buffer_size: Mutex::new(64), // Default: 64 samples
            underrun_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Get local latency info based on audio config
    fn local_latency_info(&self) -> LocalLatencyInfo {
        let frame_size = self.buffer_size.try_lock().map(|s| *s).unwrap_or(64);
        LocalLatencyInfo::from_audio_config(frame_size, AUDIO_SAMPLE_RATE, "pcm")
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
    SetMute(bool),
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

/// Audio quality metrics for IPC
#[derive(Debug, Clone, Serialize)]
pub struct AudioQuality {
    /// Number of buffer underruns (audio glitches due to CPU/scheduling)
    pub underrun_count: u64,
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
    /// Whether microphone is muted
    pub is_muted: bool,
    /// Current input audio level (0-100)
    pub input_level: u32,
    /// Network statistics
    pub network: Option<NetworkStats>,
    /// Detailed latency breakdown
    pub latency: Option<DetailedLatency>,
    /// Audio quality metrics
    pub audio_quality: Option<AudioQuality>,
}

/// Start audio streaming to a remote peer
#[tauri::command]
pub async fn streaming_start(
    remote_addr: String,
    input_device_id: Option<String>,
    output_device_id: Option<String>,
    buffer_size: u32,
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

    // Store buffer size for latency display
    {
        let mut bs = state.buffer_size.lock().await;
        *bs = buffer_size;
    }

    let is_active = state.is_active.clone();
    let is_muted = state.is_muted.clone();
    let input_level = state.input_level.clone();
    let shared_stats = state.stats.clone();
    let underrun_count = state.underrun_count.clone();

    // Reset mute state and underrun count on new connection
    state.is_muted.store(false, Ordering::SeqCst);
    state.input_level.store(0, Ordering::SeqCst);
    state.underrun_count.store(0, Ordering::SeqCst);

    // Mark as active BEFORE spawning thread to avoid race condition
    state.is_active.store(true, Ordering::SeqCst);

    // Spawn audio thread with real-time priority
    thread::spawn(move || {
        // Set real-time priority immediately upon thread start
        set_current_thread_realtime_priority();

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
                buffer_size,
                cmd_rx,
                &is_active,
                &is_muted,
                &input_level,
                &shared_stats,
                &underrun_count,
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
    let is_muted = state.is_muted.load(Ordering::SeqCst);
    let input_level = state.input_level.load(Ordering::SeqCst);
    let remote_addr = state.remote_addr.lock().await.clone();
    let stats = state.stats.read().ok().and_then(|s| s.clone());

    let (network, latency) = if let Some(ref s) = stats {
        let local_info = state.local_latency_info();
        let breakdown = LatencyBreakdown::calculate(&local_info, None, s.rtt_ms, s.jitter_ms);
        let frame_size = state.buffer_size.lock().await;

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
                    *frame_size, AUDIO_SAMPLE_RATE
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
                info: Some(format!("{} samples", *frame_size)),
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

    // Audio quality metrics
    let audio_quality = if is_active {
        Some(AudioQuality {
            underrun_count: state.underrun_count.load(Ordering::Relaxed),
        })
    } else {
        None
    };

    Ok(StreamingStatus {
        is_active,
        remote_addr,
        is_muted,
        input_level,
        network,
        latency,
        audio_quality,
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

/// Set mute state
#[tauri::command]
pub async fn streaming_set_mute(
    muted: bool,
    state: tauri::State<'_, StreamingState>,
) -> Result<(), String> {
    // Can set mute state even when not streaming
    state.is_muted.store(muted, Ordering::SeqCst);

    // If streaming, also send command to audio thread
    if state.is_active.load(Ordering::SeqCst) {
        let tx = state.cmd_tx.lock().await;
        if let Some(ref sender) = *tx {
            let _ = sender.send(StreamingCommand::SetMute(muted));
        }
    }

    Ok(())
}

/// Get mute state
#[tauri::command]
pub async fn streaming_get_mute(
    state: tauri::State<'_, StreamingState>,
) -> Result<bool, String> {
    Ok(state.is_muted.load(Ordering::SeqCst))
}

/// Get current input audio level (0-100)
#[tauri::command]
pub async fn streaming_get_input_level(
    state: tauri::State<'_, StreamingState>,
) -> Result<u32, String> {
    Ok(state.input_level.load(Ordering::SeqCst))
}

/// Run audio streaming in the audio thread
async fn run_audio_streaming(
    remote_addr: SocketAddr,
    input_device_id: Option<String>,
    output_device_id: Option<String>,
    buffer_size: u32,
    cmd_rx: std_mpsc::Receiver<StreamingCommand>,
    is_active: &AtomicBool,
    is_muted: &AtomicBool,
    input_level: &AtomicU32,
    shared_stats: &RwLock<Option<ConnectionStats>>,
    underrun_count: &AtomicU64,
) -> Result<(), String> {
    // Audio configuration - buffer_size controls latency vs stability tradeoff
    // Lower values = less latency but may cause crackling
    // Higher values = more stable but higher latency
    let config = AudioConfig {
        sample_rate: AUDIO_SAMPLE_RATE,
        channels: 1,
        frame_size: buffer_size,
    };

    // Create connection
    let mut connection = Connection::new("0.0.0.0:0")
        .await
        .map_err(|e| format!("Failed to create connection: {}", e))?;

    // Create audio engine
    let mut audio_engine = AudioEngine::new(config);

    let input_id = input_device_id.map(DeviceId);
    let output_id = output_device_id.map(DeviceId);

    // Create channels for audio data - minimal buffering for lowest latency
    // 2 slots = just enough for thread synchronization without accumulating delay
    let (tx_capture, mut rx_capture) = tokio::sync::mpsc::channel::<(Vec<f32>, u32)>(4);
    let (tx_playback, mut rx_playback) = tokio::sync::mpsc::channel::<Vec<f32>>(4);

    // Keep a clone for device switching
    let tx_capture_for_switch = tx_capture.clone();

    // Clone atomics for capture callback
    let input_level_for_capture = Arc::new(AtomicU32::new(0));
    let input_level_capture_ref = input_level_for_capture.clone();

    // Start audio capture with level metering
    audio_engine
        .start_capture(input_id.as_ref(), move |samples, timestamp| {
            // Calculate RMS level (0-100)
            if !samples.is_empty() {
                let sum_squares: f32 = samples.iter().map(|s| s * s).sum();
                let rms = (sum_squares / samples.len() as f32).sqrt();
                // Convert to 0-100 scale (assuming max amplitude of 1.0)
                // Use a slight compression curve for better visual feedback
                let level = ((rms * 100.0).min(100.0)).round() as u32;
                input_level_capture_ref.store(level, Ordering::SeqCst);
            }
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

    // Create muted state flag for send task
    let is_muted_for_send = Arc::new(AtomicBool::new(is_muted.load(Ordering::SeqCst)));
    let is_muted_send_ref = is_muted_for_send.clone();

    // Spawn task to send captured audio
    let send_task = tokio::spawn(async move {
        while let Some((samples, timestamp)) = rx_capture.recv().await {
            // Skip sending if muted
            if is_muted_send_ref.load(Ordering::SeqCst) {
                continue;
            }
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
            Ok(StreamingCommand::SetMute(muted)) => {
                println!("Setting mute state to: {}", muted);
                is_muted_for_send.store(muted, Ordering::SeqCst);
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

        // Update stats and input level every ~100ms (10 iterations * 10ms)
        stats_update_counter += 1;
        if stats_update_counter >= 10 {
            stats_update_counter = 0;
            // Update connection stats
            if let Ok(conn) = connection_arc.try_lock() {
                let conn_stats = conn.stats();
                if let Ok(mut stats) = shared_stats.write() {
                    *stats = Some(conn_stats);
                }
            }
            // Update shared input level from capture callback
            input_level.store(input_level_for_capture.load(Ordering::SeqCst), Ordering::SeqCst);
            // Update underrun count from audio engine
            underrun_count.store(audio_engine.underrun_count(), Ordering::Relaxed);
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
