//! Metronome for tempo synchronization
//!
//! Generates click sounds at a specified BPM that can be shared across peers.

use std::f32::consts::PI;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

/// Metronome configuration
#[derive(Debug, Clone)]
pub struct MetronomeConfig {
    /// Beats per minute
    pub bpm: u32,
    /// Time signature numerator (beats per measure)
    pub beats_per_measure: u32,
    /// Time signature denominator (note value that gets one beat)
    pub beat_value: u32,
    /// Click volume (0.0 - 1.0)
    pub volume: f32,
    /// Frequency of the downbeat click (Hz)
    pub downbeat_freq: f32,
    /// Frequency of other beats click (Hz)
    pub beat_freq: f32,
}

impl Default for MetronomeConfig {
    fn default() -> Self {
        Self {
            bpm: 120,
            beats_per_measure: 4,
            beat_value: 4,
            volume: 0.5,
            downbeat_freq: 1000.0,
            beat_freq: 800.0,
        }
    }
}

/// Metronome state that can be synchronized across peers
#[derive(Debug, Clone, Copy)]
pub struct MetronomeState {
    /// Current beat position (0-indexed within measure)
    pub current_beat: u32,
    /// Current measure number
    pub measure: u32,
    /// Sample position within the current beat
    pub sample_position: u64,
    /// Total samples since start
    pub total_samples: u64,
}

/// Metronome for generating synchronized clicks
pub struct Metronome {
    config: MetronomeConfig,
    sample_rate: u32,
    running: Arc<AtomicBool>,
    current_beat: AtomicU32,
    current_measure: AtomicU32,
    sample_position: Arc<std::sync::atomic::AtomicU64>,
    total_samples: Arc<std::sync::atomic::AtomicU64>,
    samples_per_beat: u32,
    click_duration_samples: u32,
}

impl Metronome {
    /// Create a new metronome
    pub fn new(config: MetronomeConfig, sample_rate: u32) -> Self {
        let samples_per_beat = (sample_rate * 60) / config.bpm;
        let click_duration_samples = sample_rate / 20; // 50ms click

        Self {
            config,
            sample_rate,
            running: Arc::new(AtomicBool::new(false)),
            current_beat: AtomicU32::new(0),
            current_measure: AtomicU32::new(0),
            sample_position: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            total_samples: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            samples_per_beat,
            click_duration_samples,
        }
    }

    /// Start the metronome
    pub fn start(&self) {
        self.running.store(true, Ordering::SeqCst);
        self.reset();
    }

    /// Stop the metronome
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Reset the metronome to the beginning
    pub fn reset(&self) {
        self.current_beat.store(0, Ordering::SeqCst);
        self.current_measure.store(0, Ordering::SeqCst);
        self.sample_position.store(0, Ordering::SeqCst);
        self.total_samples.store(0, Ordering::SeqCst);
    }

    /// Check if metronome is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Set BPM
    pub fn set_bpm(&mut self, bpm: u32) {
        self.config.bpm = bpm.clamp(20, 300);
        self.samples_per_beat = (self.sample_rate * 60) / self.config.bpm;
    }

    /// Get current BPM
    pub fn bpm(&self) -> u32 {
        self.config.bpm
    }

    /// Set volume
    pub fn set_volume(&mut self, volume: f32) {
        self.config.volume = volume.clamp(0.0, 1.0);
    }

    /// Get current state
    pub fn state(&self) -> MetronomeState {
        MetronomeState {
            current_beat: self.current_beat.load(Ordering::SeqCst),
            measure: self.current_measure.load(Ordering::SeqCst),
            sample_position: self.sample_position.load(Ordering::SeqCst),
            total_samples: self.total_samples.load(Ordering::SeqCst),
        }
    }

    /// Synchronize to a remote state
    pub fn sync_to(&self, state: MetronomeState) {
        self.current_beat
            .store(state.current_beat, Ordering::SeqCst);
        self.current_measure.store(state.measure, Ordering::SeqCst);
        self.sample_position
            .store(state.sample_position, Ordering::SeqCst);
        self.total_samples
            .store(state.total_samples, Ordering::SeqCst);
    }

    /// Generate audio samples for the metronome
    /// Returns the generated samples and advances the internal state
    pub fn generate(&self, num_samples: usize) -> Vec<f32> {
        if !self.running.load(Ordering::SeqCst) {
            return vec![0.0; num_samples];
        }

        let mut output = vec![0.0; num_samples];
        let mut sample_pos = self.sample_position.load(Ordering::SeqCst);
        let mut current_beat = self.current_beat.load(Ordering::SeqCst);
        let mut current_measure = self.current_measure.load(Ordering::SeqCst);

        for i in 0..num_samples {
            let pos_in_beat = (sample_pos % self.samples_per_beat as u64) as u32;

            // Generate click at the start of each beat
            if pos_in_beat < self.click_duration_samples {
                let is_downbeat = current_beat == 0;
                let freq = if is_downbeat {
                    self.config.downbeat_freq
                } else {
                    self.config.beat_freq
                };

                // Generate a sine wave with exponential decay
                let t = pos_in_beat as f32 / self.sample_rate as f32;
                let envelope = (-t * 30.0).exp(); // Fast decay
                let sample = (2.0 * PI * freq * t).sin() * envelope * self.config.volume;
                output[i] = sample;
            }

            // Advance position
            sample_pos += 1;

            // Check for beat transition
            if sample_pos % self.samples_per_beat as u64 == 0 {
                current_beat += 1;
                if current_beat >= self.config.beats_per_measure {
                    current_beat = 0;
                    current_measure += 1;
                }
            }
        }

        // Update state
        self.sample_position.store(sample_pos, Ordering::SeqCst);
        self.current_beat.store(current_beat, Ordering::SeqCst);
        self.current_measure
            .store(current_measure, Ordering::SeqCst);
        self.total_samples
            .fetch_add(num_samples as u64, Ordering::SeqCst);

        output
    }

    /// Mix metronome audio into an existing buffer
    pub fn mix_into(&self, buffer: &mut [f32]) {
        if !self.running.load(Ordering::SeqCst) {
            return;
        }

        let click_samples = self.generate(buffer.len());
        for (i, sample) in buffer.iter_mut().enumerate() {
            *sample += click_samples[i];
        }
    }
}

/// Message for metronome synchronization across network
#[derive(Debug, Clone, Copy)]
pub struct MetronomeSync {
    pub bpm: u32,
    pub beats_per_measure: u32,
    pub current_beat: u32,
    pub measure: u32,
    pub sample_position: u64,
}

impl MetronomeSync {
    /// Create from a metronome instance
    pub fn from_metronome(metro: &Metronome) -> Self {
        let state = metro.state();
        Self {
            bpm: metro.config.bpm,
            beats_per_measure: metro.config.beats_per_measure,
            current_beat: state.current_beat,
            measure: state.measure,
            sample_position: state.sample_position,
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(24);
        bytes.extend_from_slice(&self.bpm.to_be_bytes());
        bytes.extend_from_slice(&self.beats_per_measure.to_be_bytes());
        bytes.extend_from_slice(&self.current_beat.to_be_bytes());
        bytes.extend_from_slice(&self.measure.to_be_bytes());
        bytes.extend_from_slice(&self.sample_position.to_be_bytes());
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 24 {
            return None;
        }
        Some(Self {
            bpm: u32::from_be_bytes([data[0], data[1], data[2], data[3]]),
            beats_per_measure: u32::from_be_bytes([data[4], data[5], data[6], data[7]]),
            current_beat: u32::from_be_bytes([data[8], data[9], data[10], data[11]]),
            measure: u32::from_be_bytes([data[12], data[13], data[14], data[15]]),
            sample_position: u64::from_be_bytes([
                data[16], data[17], data[18], data[19], data[20], data[21], data[22], data[23],
            ]),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metronome_creation() {
        let config = MetronomeConfig::default();
        let metro = Metronome::new(config, 48000);

        assert!(!metro.is_running());
        assert_eq!(metro.bpm(), 120);
    }

    #[test]
    fn test_metronome_start_stop() {
        let metro = Metronome::new(MetronomeConfig::default(), 48000);

        metro.start();
        assert!(metro.is_running());

        metro.stop();
        assert!(!metro.is_running());
    }

    #[test]
    fn test_metronome_generate() {
        let metro = Metronome::new(MetronomeConfig::default(), 48000);
        metro.start();

        let samples = metro.generate(1024);
        assert_eq!(samples.len(), 1024);

        // First samples should have non-zero values (click sound)
        assert!(samples.iter().take(100).any(|&s| s.abs() > 0.01));
    }

    #[test]
    fn test_metronome_sync() {
        let metro = Metronome::new(MetronomeConfig::default(), 48000);
        metro.start();
        metro.generate(10000); // Advance

        let sync = MetronomeSync::from_metronome(&metro);
        let bytes = sync.to_bytes();
        let parsed = MetronomeSync::from_bytes(&bytes).unwrap();

        assert_eq!(sync.bpm, parsed.bpm);
        assert_eq!(sync.current_beat, parsed.current_beat);
    }

    #[test]
    fn test_set_bpm() {
        let mut metro = Metronome::new(MetronomeConfig::default(), 48000);

        metro.set_bpm(140);
        assert_eq!(metro.bpm(), 140);

        // Test clamping
        metro.set_bpm(10);
        assert_eq!(metro.bpm(), 20);

        metro.set_bpm(400);
        assert_eq!(metro.bpm(), 300);
    }
}
