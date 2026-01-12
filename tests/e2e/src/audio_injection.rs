//! Audio injection and capture for E2E testing
//!
//! Provides utilities for injecting test audio into virtual audio devices
//! and capturing received audio for quality evaluation.

use hound::{WavReader, WavSpec, WavWriter};
use std::path::Path;
use tracing::{debug, info};

/// Audio injector for virtual audio devices
pub struct AudioInjector {
    /// Sample rate
    sample_rate: u32,
    /// Number of channels
    channels: u16,
}

impl AudioInjector {
    /// Create a new audio injector
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        Self {
            sample_rate,
            channels,
        }
    }

    /// Load audio samples from a WAV file
    pub fn load_wav<P: AsRef<Path>>(path: P) -> Result<Vec<f32>, AudioError> {
        let path = path.as_ref();
        info!("Loading WAV file: {}", path.display());

        let reader = WavReader::open(path).map_err(|e| AudioError::FileOpen(e.to_string()))?;

        let spec = reader.spec();
        debug!(
            "WAV spec: {} Hz, {} channels, {} bits",
            spec.sample_rate, spec.channels, spec.bits_per_sample
        );

        // Convert samples to f32
        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => reader
                .into_samples::<f32>()
                .filter_map(|s| s.ok())
                .collect(),
            hound::SampleFormat::Int => {
                let max_val = (1 << (spec.bits_per_sample - 1)) as f32;
                reader
                    .into_samples::<i32>()
                    .filter_map(|s| s.ok())
                    .map(|s| s as f32 / max_val)
                    .collect()
            }
        };

        info!("Loaded {} samples from WAV", samples.len());
        Ok(samples)
    }

    /// Save audio samples to a WAV file
    pub fn save_wav<P: AsRef<Path>>(
        path: P,
        samples: &[f32],
        sample_rate: u32,
        channels: u16,
    ) -> Result<(), AudioError> {
        let path = path.as_ref();
        info!("Saving WAV file: {}", path.display());

        let spec = WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        let mut writer =
            WavWriter::create(path, spec).map_err(|e| AudioError::FileWrite(e.to_string()))?;

        for &sample in samples {
            writer
                .write_sample(sample)
                .map_err(|e| AudioError::FileWrite(e.to_string()))?;
        }

        writer
            .finalize()
            .map_err(|e| AudioError::FileWrite(e.to_string()))?;

        info!("Saved {} samples to WAV", samples.len());
        Ok(())
    }

    /// Generate a test tone (sine wave)
    pub fn generate_sine(&self, frequency: f32, duration_sec: f32) -> Vec<f32> {
        let num_samples = (self.sample_rate as f32 * duration_sec) as usize;
        let mut samples = Vec::with_capacity(num_samples * self.channels as usize);

        for i in 0..num_samples {
            let t = i as f32 / self.sample_rate as f32;
            let sample = (2.0 * std::f32::consts::PI * frequency * t).sin() * 0.5;

            // Write same sample to all channels
            for _ in 0..self.channels {
                samples.push(sample);
            }
        }

        samples
    }

    /// Generate a frequency sweep for codec testing
    pub fn generate_sweep(
        &self,
        start_freq: f32,
        end_freq: f32,
        duration_sec: f32,
    ) -> Vec<f32> {
        let num_samples = (self.sample_rate as f32 * duration_sec) as usize;
        let mut samples = Vec::with_capacity(num_samples * self.channels as usize);

        for i in 0..num_samples {
            let t = i as f32 / num_samples as f32;
            let freq = start_freq + (end_freq - start_freq) * t;
            let phase = 2.0 * std::f32::consts::PI * freq * (i as f32 / self.sample_rate as f32);
            let sample = phase.sin() * 0.5;

            for _ in 0..self.channels {
                samples.push(sample);
            }
        }

        samples
    }

    /// Generate silence
    pub fn generate_silence(&self, duration_sec: f32) -> Vec<f32> {
        let num_samples = (self.sample_rate as f32 * duration_sec) as usize;
        vec![0.0; num_samples * self.channels as usize]
    }
}

/// Audio capture for recording received audio
pub struct AudioCapture {
    /// Sample rate
    sample_rate: u32,
    /// Number of channels
    channels: u16,
    /// Captured samples
    samples: Vec<f32>,
}

impl AudioCapture {
    /// Create a new audio capture
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        Self {
            sample_rate,
            channels,
            samples: Vec::new(),
        }
    }

    /// Add samples to the capture buffer
    pub fn push_samples(&mut self, samples: &[f32]) {
        self.samples.extend_from_slice(samples);
    }

    /// Get the captured samples
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    /// Get the duration of captured audio in seconds
    pub fn duration_sec(&self) -> f32 {
        self.samples.len() as f32 / (self.sample_rate as f32 * self.channels as f32)
    }

    /// Save captured audio to a WAV file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), AudioError> {
        AudioInjector::save_wav(path, &self.samples, self.sample_rate, self.channels)
    }

    /// Clear the capture buffer
    pub fn clear(&mut self) {
        self.samples.clear();
    }
}

/// Errors that can occur during audio operations
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("Failed to open file: {0}")]
    FileOpen(String),

    #[error("Failed to write file: {0}")]
    FileWrite(String),

    #[error("Invalid audio format: {0}")]
    InvalidFormat(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_sine() {
        let injector = AudioInjector::new(48000, 1);
        let samples = injector.generate_sine(440.0, 0.1);

        // 48000 Hz * 0.1 sec = 4800 samples
        assert_eq!(samples.len(), 4800);

        // Check that samples are in valid range
        for sample in &samples {
            assert!(*sample >= -1.0 && *sample <= 1.0);
        }
    }

    #[test]
    fn test_generate_sweep() {
        let injector = AudioInjector::new(48000, 2);
        let samples = injector.generate_sweep(100.0, 10000.0, 0.5);

        // 48000 Hz * 0.5 sec * 2 channels = 48000 samples
        assert_eq!(samples.len(), 48000);
    }

    #[test]
    fn test_generate_silence() {
        let injector = AudioInjector::new(48000, 1);
        let samples = injector.generate_silence(0.1);

        assert_eq!(samples.len(), 4800);
        for sample in &samples {
            assert_eq!(*sample, 0.0);
        }
    }

    #[test]
    fn test_audio_capture() {
        let mut capture = AudioCapture::new(48000, 1);
        capture.push_samples(&[0.1, 0.2, 0.3]);
        capture.push_samples(&[0.4, 0.5]);

        assert_eq!(capture.samples().len(), 5);
        assert!((capture.duration_sec() - 5.0 / 48000.0).abs() < 0.0001);
    }
}
