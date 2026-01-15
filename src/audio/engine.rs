//! Audio engine for capture and playback

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Stream, StreamConfig};
use ringbuf::traits::{Consumer, Producer, Split};
use ringbuf::HeapRb;
use tracing::{debug, error, info, warn};

use super::device::DeviceId;
use super::error::AudioError;

/// Events that can occur during audio streaming
#[derive(Debug, Clone)]
pub enum AudioEvent {
    /// Input device was disconnected
    InputDeviceDisconnected,
    /// Output device was disconnected
    OutputDeviceDisconnected,
    /// Stream error occurred
    StreamError(String),
}

/// Bit depth for audio samples
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BitDepth {
    /// 16-bit signed integer
    I16,
    /// 24-bit signed integer
    I24,
    /// 32-bit floating point
    #[default]
    F32,
}

/// Audio configuration (shared base)
#[derive(Debug, Clone)]
pub struct AudioConfig {
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Number of channels
    pub channels: u16,
    /// Frame size in samples
    pub frame_size: u32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            channels: 1,
            frame_size: 64,
        }
    }
}

/// Capture configuration
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Number of channels
    pub channels: u16,
    /// Frame size in samples
    pub frame_size: u32,
    /// Bit depth
    pub bit_depth: BitDepth,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            channels: 1,
            frame_size: 64,
            bit_depth: BitDepth::F32,
        }
    }
}

impl From<CaptureConfig> for AudioConfig {
    fn from(config: CaptureConfig) -> Self {
        AudioConfig {
            sample_rate: config.sample_rate,
            channels: config.channels,
            frame_size: config.frame_size,
        }
    }
}

/// Playback configuration
#[derive(Debug, Clone)]
pub struct PlaybackConfig {
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Number of channels
    pub channels: u16,
    /// Frame size in samples
    pub frame_size: u32,
    /// Bit depth
    pub bit_depth: BitDepth,
}

impl Default for PlaybackConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            channels: 1,
            frame_size: 64,
            bit_depth: BitDepth::F32,
        }
    }
}

impl From<PlaybackConfig> for AudioConfig {
    fn from(config: PlaybackConfig) -> Self {
        AudioConfig {
            sample_rate: config.sample_rate,
            channels: config.channels,
            frame_size: config.frame_size,
        }
    }
}

/// Audio buffer for samples
#[derive(Debug, Clone)]
pub struct AudioBuffer {
    /// Sample data (interleaved format)
    pub data: Vec<f32>,
    /// Number of channels
    pub channels: u16,
    /// Number of samples per channel
    pub samples: u32,
}

impl AudioBuffer {
    /// Create a new audio buffer
    pub fn new(data: Vec<f32>, channels: u16) -> Self {
        let samples = if channels > 0 {
            (data.len() / channels as usize) as u32
        } else {
            0
        };
        Self {
            data,
            channels,
            samples,
        }
    }

    /// Create an empty buffer
    pub fn empty(channels: u16, samples: u32) -> Self {
        Self {
            data: vec![0.0; (channels as usize) * (samples as usize)],
            channels,
            samples,
        }
    }
}

/// Callback type for captured audio data
#[allow(dead_code)]
pub type CaptureCallback = Box<dyn Fn(&[f32], u64) + Send + 'static>;

/// Thread-safe playback producer wrapper
pub type SharedPlaybackProducer = Arc<std::sync::Mutex<ringbuf::HeapProd<f32>>>;

/// Audio engine handles capture and playback
pub struct AudioEngine {
    config: AudioConfig,
    capture_stream: Option<Stream>,
    playback_stream: Option<Stream>,
    running: Arc<AtomicBool>,
    // Ring buffer for playback: producer is filled by network/monitoring, consumer is read by audio thread
    playback_producer: Option<SharedPlaybackProducer>,
    // Local monitoring
    local_monitor_enabled: Arc<AtomicBool>,
    // Current device IDs (None = default device)
    current_input_device: Option<DeviceId>,
    current_output_device: Option<DeviceId>,
    // Event sender for device change notifications
    event_tx: Option<Sender<AudioEvent>>,
}

impl AudioEngine {
    /// Create a new audio engine with the given configuration
    pub fn new(config: AudioConfig) -> Self {
        Self {
            config,
            capture_stream: None,
            playback_stream: None,
            running: Arc::new(AtomicBool::new(false)),
            playback_producer: None,
            local_monitor_enabled: Arc::new(AtomicBool::new(false)),
            current_input_device: None,
            current_output_device: None,
            event_tx: None,
        }
    }

    /// Set event sender for device change notifications
    pub fn set_event_sender(&mut self, tx: Sender<AudioEvent>) {
        self.event_tx = Some(tx);
    }

    /// Get current input device ID
    pub fn current_input_device(&self) -> Option<&DeviceId> {
        self.current_input_device.as_ref()
    }

    /// Get current output device ID
    pub fn current_output_device(&self) -> Option<&DeviceId> {
        self.current_output_device.as_ref()
    }

    /// Get the shared playback producer for external use (e.g., local monitoring)
    pub fn playback_producer(&self) -> Option<SharedPlaybackProducer> {
        self.playback_producer.clone()
    }

    /// Get the local monitoring flag for use in capture callbacks
    pub fn local_monitor_flag(&self) -> Arc<AtomicBool> {
        self.local_monitor_enabled.clone()
    }

    /// Start audio capture with a callback for captured samples
    pub fn start_capture<F>(
        &mut self,
        device_id: Option<&DeviceId>,
        callback: F,
    ) -> Result<(), AudioError>
    where
        F: Fn(&[f32], u64) + Send + Sync + 'static,
    {
        let host = cpal::default_host();

        let device = match device_id {
            Some(id) => host
                .input_devices()
                .map_err(|e| AudioError::DeviceOpenFailed(e.to_string()))?
                .find(|d| d.name().ok().as_ref() == Some(&id.0))
                .ok_or_else(|| AudioError::DeviceNotFound(id.0.clone()))?,
            None => host
                .default_input_device()
                .ok_or_else(|| AudioError::DeviceNotFound("No default input device".into()))?,
        };

        let device_name = device.name().unwrap_or_default();
        info!("Starting capture on device: {}", device_name);

        // Store current device ID
        self.current_input_device = device_id.cloned();

        let stream_config = StreamConfig {
            channels: self.config.channels,
            sample_rate: cpal::SampleRate(self.config.sample_rate),
            buffer_size: cpal::BufferSize::Fixed(self.config.frame_size),
        };

        let sample_count = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let sample_count_clone = sample_count.clone();
        let callback = Arc::new(callback);
        let callback_clone = callback.clone();

        // Error callback with device disconnection detection
        let event_tx = self.event_tx.clone();
        let err_fn = move |err: cpal::StreamError| {
            error!("Capture stream error: {:?}", err);
            match err {
                cpal::StreamError::DeviceNotAvailable => {
                    warn!("Input device disconnected");
                    if let Some(ref tx) = event_tx {
                        let _ = tx.send(AudioEvent::InputDeviceDisconnected);
                    }
                }
                _ => {
                    if let Some(ref tx) = event_tx {
                        let _ = tx.send(AudioEvent::StreamError(err.to_string()));
                    }
                }
            }
        };

        let stream = device
            .build_input_stream(
                &stream_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let timestamp =
                        sample_count_clone.fetch_add(data.len() as u64, Ordering::Relaxed);
                    callback_clone(data, timestamp);
                },
                err_fn,
                None,
            )
            .map_err(|e| AudioError::StreamError(e.to_string()))?;

        stream
            .play()
            .map_err(|e| AudioError::StreamError(e.to_string()))?;
        self.capture_stream = Some(stream);
        self.running.store(true, Ordering::SeqCst);

        debug!("Capture started with config: {:?}", self.config);
        Ok(())
    }

    /// Start audio playback
    pub fn start_playback(&mut self, device_id: Option<&DeviceId>) -> Result<(), AudioError> {
        let host = cpal::default_host();

        let device = match device_id {
            Some(id) => host
                .output_devices()
                .map_err(|e| AudioError::DeviceOpenFailed(e.to_string()))?
                .find(|d| d.name().ok().as_ref() == Some(&id.0))
                .ok_or_else(|| AudioError::DeviceNotFound(id.0.clone()))?,
            None => host
                .default_output_device()
                .ok_or_else(|| AudioError::DeviceNotFound("No default output device".into()))?,
        };

        let device_name = device.name().unwrap_or_default();
        info!("Starting playback on device: {}", device_name);

        // Store current device ID
        self.current_output_device = device_id.cloned();

        let stream_config = StreamConfig {
            channels: self.config.channels,
            sample_rate: cpal::SampleRate(self.config.sample_rate),
            buffer_size: cpal::BufferSize::Fixed(self.config.frame_size),
        };

        // Create ring buffer for playback - minimal size for lowest latency
        // 3 frames to prevent underruns
        let buffer_size = (self.config.frame_size * self.config.channels as u32 * 3) as usize;
        let rb = HeapRb::<f32>::new(buffer_size);
        let (producer, consumer) = rb.split();
        // Wrap producer in Arc<Mutex<>> for sharing with capture callback
        self.playback_producer = Some(Arc::new(std::sync::Mutex::new(producer)));

        let consumer = Arc::new(std::sync::Mutex::new(consumer));
        let consumer_clone = consumer.clone();

        // Error callback with device disconnection detection
        let event_tx = self.event_tx.clone();
        let err_fn = move |err: cpal::StreamError| {
            error!("Playback stream error: {:?}", err);
            match err {
                cpal::StreamError::DeviceNotAvailable => {
                    warn!("Output device disconnected");
                    if let Some(ref tx) = event_tx {
                        let _ = tx.send(AudioEvent::OutputDeviceDisconnected);
                    }
                }
                _ => {
                    if let Some(ref tx) = event_tx {
                        let _ = tx.send(AudioEvent::StreamError(err.to_string()));
                    }
                }
            }
        };

        let stream = device
            .build_output_stream(
                &stream_config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    // Use try_lock to avoid blocking in real-time audio callback
                    if let Ok(mut cons) = consumer_clone.try_lock() {
                        // Read samples from ring buffer
                        for sample in data.iter_mut() {
                            *sample = cons.try_pop().unwrap_or(0.0);
                        }
                    } else {
                        // Lock not available, output silence to avoid blocking
                        for sample in data.iter_mut() {
                            *sample = 0.0;
                        }
                    }
                },
                err_fn,
                None,
            )
            .map_err(|e| AudioError::StreamError(e.to_string()))?;

        stream
            .play()
            .map_err(|e| AudioError::StreamError(e.to_string()))?;
        self.playback_stream = Some(stream);

        debug!("Playback started with config: {:?}", self.config);
        Ok(())
    }

    /// Enqueue audio samples for playback
    /// Returns the number of samples actually enqueued
    pub fn enqueue_playback(&self, samples: &[f32]) -> usize {
        if let Some(ref producer) = self.playback_producer {
            if let Ok(mut prod) = producer.try_lock() {
                let mut count = 0;
                for &sample in samples {
                    if prod.try_push(sample).is_ok() {
                        count += 1;
                    } else {
                        break;
                    }
                }
                count
            } else {
                0
            }
        } else {
            0
        }
    }

    /// Stop capture
    pub fn stop_capture(&mut self) {
        self.capture_stream = None;
        info!("Capture stopped");
    }

    /// Stop playback
    pub fn stop_playback(&mut self) {
        self.playback_stream = None;
        self.playback_producer = None;
        info!("Playback stopped");
    }

    /// Enable or disable local monitoring
    pub fn set_local_monitoring(&self, enabled: bool) {
        self.local_monitor_enabled.store(enabled, Ordering::SeqCst);
        info!(
            "Local monitoring: {}",
            if enabled { "enabled" } else { "disabled" }
        );
    }

    /// Check if local monitoring is enabled
    pub fn is_local_monitoring_enabled(&self) -> bool {
        self.local_monitor_enabled.load(Ordering::SeqCst)
    }

    /// Get current configuration
    pub fn config(&self) -> &AudioConfig {
        &self.config
    }

    /// Switch input device while running
    ///
    /// Thread: Must be called from non-realtime thread
    /// Blocking: Yes (until new stream is ready)
    ///
    /// This stops the existing capture stream and starts a new one with the new device.
    /// There will be a brief audio gap (~10-50ms) during the switch.
    pub fn set_input_device<F>(
        &mut self,
        device_id: Option<&DeviceId>,
        callback: F,
    ) -> Result<(), AudioError>
    where
        F: Fn(&[f32], u64) + Send + Sync + 'static,
    {
        info!("Switching input device to: {:?}", device_id.map(|d| &d.0));

        // Stop existing capture stream
        self.stop_capture();

        // Start new capture with new device
        self.start_capture(device_id, callback)
    }

    /// Switch output device while running
    ///
    /// Thread: Must be called from non-realtime thread
    /// Blocking: Yes (until new stream is ready)
    ///
    /// This stops the existing playback stream and starts a new one with the new device.
    /// There will be a brief audio gap (~10-50ms) during the switch.
    pub fn set_output_device(&mut self, device_id: Option<&DeviceId>) -> Result<(), AudioError> {
        info!("Switching output device to: {:?}", device_id.map(|d| &d.0));

        // Check if playback was running
        let was_running = self.playback_stream.is_some();

        // Stop existing playback stream
        self.stop_playback();

        // Start new playback with new device if it was running
        if was_running {
            self.start_playback(device_id)?;
        }

        Ok(())
    }

    /// Check if capture is currently running
    pub fn is_capture_running(&self) -> bool {
        self.capture_stream.is_some()
    }

    /// Check if playback is currently running
    pub fn is_playback_running(&self) -> bool {
        self.playback_stream.is_some()
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        self.stop_capture();
        self.stop_playback();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_config_default() {
        let config = AudioConfig::default();
        assert_eq!(config.sample_rate, 48000);
        assert_eq!(config.channels, 1);
        assert_eq!(config.frame_size, 64);
    }

    #[test]
    fn test_engine_creation() {
        let config = AudioConfig::default();
        let engine = AudioEngine::new(config.clone());
        assert_eq!(engine.config().sample_rate, config.sample_rate);
    }

    #[test]
    fn test_local_monitoring_toggle() {
        let engine = AudioEngine::new(AudioConfig::default());
        assert!(!engine.is_local_monitoring_enabled());

        engine.set_local_monitoring(true);
        assert!(engine.is_local_monitoring_enabled());

        engine.set_local_monitoring(false);
        assert!(!engine.is_local_monitoring_enabled());
    }
}
