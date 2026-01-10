//! Audio engine for capture and playback

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Stream, StreamConfig};
use ringbuf::traits::{Consumer, Producer, Split};
use ringbuf::HeapRb;
use tracing::{debug, error, info};

use super::device::DeviceId;
use super::error::AudioError;

/// Bit depth for audio samples
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitDepth {
    /// 16-bit signed integer
    I16,
    /// 24-bit signed integer
    I24,
    /// 32-bit floating point
    F32,
}

impl Default for BitDepth {
    fn default() -> Self {
        BitDepth::F32
    }
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
            frame_size: 128,
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
            frame_size: 128,
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
            frame_size: 128,
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
        Self { data, channels, samples }
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

/// Audio engine handles capture and playback
pub struct AudioEngine {
    config: AudioConfig,
    capture_stream: Option<Stream>,
    playback_stream: Option<Stream>,
    running: Arc<AtomicBool>,
    // Ring buffer for playback: producer is filled by network, consumer is read by audio thread
    playback_producer: Option<ringbuf::HeapProd<f32>>,
    #[allow(dead_code)]
    playback_consumer: Option<ringbuf::HeapCons<f32>>,
    // Local monitoring
    local_monitor_enabled: Arc<AtomicBool>,
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
            playback_consumer: None,
            local_monitor_enabled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start audio capture with a callback for captured samples
    pub fn start_capture<F>(&mut self, device_id: Option<&DeviceId>, callback: F) -> Result<(), AudioError>
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

        let stream_config = StreamConfig {
            channels: self.config.channels,
            sample_rate: cpal::SampleRate(self.config.sample_rate),
            buffer_size: cpal::BufferSize::Fixed(self.config.frame_size),
        };

        let sample_count = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let sample_count_clone = sample_count.clone();
        let callback = Arc::new(callback);
        let callback_clone = callback.clone();

        let err_fn = |err| error!("Capture stream error: {}", err);

        let stream = device
            .build_input_stream(
                &stream_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let timestamp = sample_count_clone.fetch_add(data.len() as u64, Ordering::Relaxed);
                    callback_clone(data, timestamp);
                },
                err_fn,
                None,
            )
            .map_err(|e| AudioError::StreamError(e.to_string()))?;

        stream.play().map_err(|e| AudioError::StreamError(e.to_string()))?;
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

        let stream_config = StreamConfig {
            channels: self.config.channels,
            sample_rate: cpal::SampleRate(self.config.sample_rate),
            buffer_size: cpal::BufferSize::Fixed(self.config.frame_size),
        };

        // Create ring buffer for playback (10 frames worth of buffer)
        let buffer_size = (self.config.frame_size * self.config.channels as u32 * 10) as usize;
        let rb = HeapRb::<f32>::new(buffer_size);
        let (producer, consumer) = rb.split();
        self.playback_producer = Some(producer);

        let consumer = Arc::new(std::sync::Mutex::new(consumer));
        let consumer_clone = consumer.clone();

        let err_fn = |err| error!("Playback stream error: {}", err);

        let stream = device
            .build_output_stream(
                &stream_config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let mut cons = consumer_clone.lock().unwrap();
                    // Read samples from ring buffer
                    for sample in data.iter_mut() {
                        *sample = cons.try_pop().unwrap_or(0.0);
                    }
                },
                err_fn,
                None,
            )
            .map_err(|e| AudioError::StreamError(e.to_string()))?;

        stream.play().map_err(|e| AudioError::StreamError(e.to_string()))?;
        self.playback_stream = Some(stream);

        debug!("Playback started with config: {:?}", self.config);
        Ok(())
    }

    /// Enqueue audio samples for playback
    /// Returns the number of samples actually enqueued
    pub fn enqueue_playback(&mut self, samples: &[f32]) -> usize {
        if let Some(ref mut producer) = self.playback_producer {
            let mut count = 0;
            for &sample in samples {
                if producer.try_push(sample).is_ok() {
                    count += 1;
                } else {
                    break;
                }
            }
            count
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
        info!("Local monitoring: {}", if enabled { "enabled" } else { "disabled" });
    }

    /// Check if local monitoring is enabled
    pub fn is_local_monitoring_enabled(&self) -> bool {
        self.local_monitor_enabled.load(Ordering::SeqCst)
    }

    /// Get current configuration
    pub fn config(&self) -> &AudioConfig {
        &self.config
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
        assert_eq!(config.frame_size, 128);
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
