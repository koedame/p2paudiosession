//! Audio effects processing
//!
//! Provides basic audio effects for real-time processing.

use std::f32::consts::PI;

/// Effect trait for audio processing
pub trait Effect: Send + Sync {
    /// Process audio samples in place
    fn process(&mut self, samples: &mut [f32]);

    /// Reset effect state
    fn reset(&mut self);

    /// Get effect name
    fn name(&self) -> &str;

    /// Check if effect is enabled
    fn is_enabled(&self) -> bool;

    /// Enable or disable effect
    fn set_enabled(&mut self, enabled: bool);
}

/// Gain effect (volume adjustment)
pub struct Gain {
    enabled: bool,
    /// Gain in linear scale (1.0 = unity)
    pub gain: f32,
}

impl Gain {
    pub fn new(gain_db: f32) -> Self {
        Self {
            enabled: true,
            gain: db_to_linear(gain_db),
        }
    }

    pub fn set_gain_db(&mut self, db: f32) {
        self.gain = db_to_linear(db);
    }
}

impl Effect for Gain {
    fn process(&mut self, samples: &mut [f32]) {
        if !self.enabled {
            return;
        }
        for sample in samples.iter_mut() {
            *sample *= self.gain;
        }
    }

    fn reset(&mut self) {}

    fn name(&self) -> &str {
        "Gain"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

/// Simple low-pass filter
pub struct LowPassFilter {
    enabled: bool,
    cutoff: f32,
    sample_rate: f32,
    prev_sample: f32,
    alpha: f32,
}

impl LowPassFilter {
    pub fn new(cutoff_hz: f32, sample_rate: f32) -> Self {
        let mut filter = Self {
            enabled: true,
            cutoff: cutoff_hz,
            sample_rate,
            prev_sample: 0.0,
            alpha: 0.0,
        };
        filter.update_alpha();
        filter
    }

    pub fn set_cutoff(&mut self, cutoff_hz: f32) {
        self.cutoff = cutoff_hz.clamp(20.0, self.sample_rate / 2.0);
        self.update_alpha();
    }

    fn update_alpha(&mut self) {
        let rc = 1.0 / (2.0 * PI * self.cutoff);
        let dt = 1.0 / self.sample_rate;
        self.alpha = dt / (rc + dt);
    }
}

impl Effect for LowPassFilter {
    fn process(&mut self, samples: &mut [f32]) {
        if !self.enabled {
            return;
        }
        for sample in samples.iter_mut() {
            self.prev_sample = self.prev_sample + self.alpha * (*sample - self.prev_sample);
            *sample = self.prev_sample;
        }
    }

    fn reset(&mut self) {
        self.prev_sample = 0.0;
    }

    fn name(&self) -> &str {
        "Low-Pass Filter"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

/// Simple high-pass filter
pub struct HighPassFilter {
    enabled: bool,
    cutoff: f32,
    sample_rate: f32,
    prev_input: f32,
    prev_output: f32,
    alpha: f32,
}

impl HighPassFilter {
    pub fn new(cutoff_hz: f32, sample_rate: f32) -> Self {
        let mut filter = Self {
            enabled: true,
            cutoff: cutoff_hz,
            sample_rate,
            prev_input: 0.0,
            prev_output: 0.0,
            alpha: 0.0,
        };
        filter.update_alpha();
        filter
    }

    pub fn set_cutoff(&mut self, cutoff_hz: f32) {
        self.cutoff = cutoff_hz.clamp(20.0, self.sample_rate / 2.0);
        self.update_alpha();
    }

    fn update_alpha(&mut self) {
        let rc = 1.0 / (2.0 * PI * self.cutoff);
        let dt = 1.0 / self.sample_rate;
        self.alpha = rc / (rc + dt);
    }
}

impl Effect for HighPassFilter {
    fn process(&mut self, samples: &mut [f32]) {
        if !self.enabled {
            return;
        }
        for sample in samples.iter_mut() {
            let input = *sample;
            self.prev_output = self.alpha * (self.prev_output + input - self.prev_input);
            self.prev_input = input;
            *sample = self.prev_output;
        }
    }

    fn reset(&mut self) {
        self.prev_input = 0.0;
        self.prev_output = 0.0;
    }

    fn name(&self) -> &str {
        "High-Pass Filter"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

/// Simple compressor
pub struct Compressor {
    enabled: bool,
    /// Threshold in dB
    pub threshold_db: f32,
    /// Ratio (e.g., 4.0 = 4:1 compression)
    pub ratio: f32,
    /// Attack time in ms
    pub attack_ms: f32,
    /// Release time in ms
    pub release_ms: f32,
    /// Makeup gain in dB
    pub makeup_db: f32,

    sample_rate: f32,
    envelope: f32,
}

impl Compressor {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            enabled: true,
            threshold_db: -20.0,
            ratio: 4.0,
            attack_ms: 10.0,
            release_ms: 100.0,
            makeup_db: 0.0,
            sample_rate,
            envelope: 0.0,
        }
    }
}

impl Effect for Compressor {
    fn process(&mut self, samples: &mut [f32]) {
        if !self.enabled {
            return;
        }

        let attack_coeff = (-1.0 / (self.attack_ms * 0.001 * self.sample_rate)).exp();
        let release_coeff = (-1.0 / (self.release_ms * 0.001 * self.sample_rate)).exp();
        let threshold = db_to_linear(self.threshold_db);
        let makeup = db_to_linear(self.makeup_db);

        for sample in samples.iter_mut() {
            let input_abs = sample.abs();

            // Envelope follower
            if input_abs > self.envelope {
                self.envelope = attack_coeff * self.envelope + (1.0 - attack_coeff) * input_abs;
            } else {
                self.envelope = release_coeff * self.envelope + (1.0 - release_coeff) * input_abs;
            }

            // Gain computation
            let gain = if self.envelope > threshold {
                let over_db = linear_to_db(self.envelope / threshold);
                let compressed_db = over_db / self.ratio;
                db_to_linear(compressed_db - over_db)
            } else {
                1.0
            };

            *sample *= gain * makeup;
        }
    }

    fn reset(&mut self) {
        self.envelope = 0.0;
    }

    fn name(&self) -> &str {
        "Compressor"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

/// Simple delay effect
pub struct Delay {
    enabled: bool,
    /// Delay time in ms
    pub delay_ms: f32,
    /// Feedback amount (0.0 - 0.95)
    pub feedback: f32,
    /// Wet/dry mix (0.0 = dry, 1.0 = wet)
    pub mix: f32,

    buffer: Vec<f32>,
    write_pos: usize,
    sample_rate: f32,
}

impl Delay {
    pub fn new(delay_ms: f32, sample_rate: f32) -> Self {
        let buffer_size = (delay_ms * 0.001 * sample_rate) as usize + 1;
        Self {
            enabled: true,
            delay_ms,
            feedback: 0.3,
            mix: 0.5,
            buffer: vec![0.0; buffer_size],
            write_pos: 0,
            sample_rate,
        }
    }

    pub fn set_delay(&mut self, delay_ms: f32) {
        self.delay_ms = delay_ms;
        let new_size = (delay_ms * 0.001 * self.sample_rate) as usize + 1;
        if new_size != self.buffer.len() {
            self.buffer.resize(new_size, 0.0);
            self.write_pos %= self.buffer.len();
        }
    }
}

impl Effect for Delay {
    fn process(&mut self, samples: &mut [f32]) {
        if !self.enabled || self.buffer.is_empty() {
            return;
        }

        let feedback = self.feedback.clamp(0.0, 0.95);

        for sample in samples.iter_mut() {
            let delayed = self.buffer[self.write_pos];
            let input = *sample + delayed * feedback;
            self.buffer[self.write_pos] = input;

            *sample = *sample * (1.0 - self.mix) + delayed * self.mix;

            self.write_pos = (self.write_pos + 1) % self.buffer.len();
        }
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_pos = 0;
    }

    fn name(&self) -> &str {
        "Delay"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

/// Noise gate
pub struct NoiseGate {
    enabled: bool,
    /// Threshold in dB
    pub threshold_db: f32,
    /// Attack time in ms
    pub attack_ms: f32,
    /// Release time in ms
    pub release_ms: f32,

    sample_rate: f32,
    gate_open: f32, // 0.0 = closed, 1.0 = open
}

impl NoiseGate {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            enabled: true,
            threshold_db: -40.0,
            attack_ms: 1.0,
            release_ms: 50.0,
            sample_rate,
            gate_open: 0.0,
        }
    }
}

impl Effect for NoiseGate {
    fn process(&mut self, samples: &mut [f32]) {
        if !self.enabled {
            return;
        }

        let threshold = db_to_linear(self.threshold_db);
        let attack_coeff = 1.0 - (-1.0 / (self.attack_ms * 0.001 * self.sample_rate)).exp();
        let release_coeff = 1.0 - (-1.0 / (self.release_ms * 0.001 * self.sample_rate)).exp();

        for sample in samples.iter_mut() {
            let target = if sample.abs() > threshold { 1.0 } else { 0.0 };

            if target > self.gate_open {
                self.gate_open += attack_coeff * (target - self.gate_open);
            } else {
                self.gate_open += release_coeff * (target - self.gate_open);
            }

            *sample *= self.gate_open;
        }
    }

    fn reset(&mut self) {
        self.gate_open = 0.0;
    }

    fn name(&self) -> &str {
        "Noise Gate"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

/// Effect chain for processing multiple effects
pub struct EffectChain {
    effects: Vec<Box<dyn Effect>>,
    enabled: bool,
}

impl EffectChain {
    pub fn new() -> Self {
        Self {
            effects: Vec::new(),
            enabled: true,
        }
    }

    pub fn add(&mut self, effect: Box<dyn Effect>) {
        self.effects.push(effect);
    }

    pub fn remove(&mut self, index: usize) -> Option<Box<dyn Effect>> {
        if index < self.effects.len() {
            Some(self.effects.remove(index))
        } else {
            None
        }
    }

    pub fn get(&self, index: usize) -> Option<&dyn Effect> {
        self.effects.get(index).map(|b| b.as_ref())
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Box<dyn Effect>> {
        self.effects.get_mut(index)
    }

    pub fn len(&self) -> usize {
        self.effects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    pub fn process(&mut self, samples: &mut [f32]) {
        if !self.enabled {
            return;
        }
        for effect in &mut self.effects {
            effect.process(samples);
        }
    }

    pub fn reset(&mut self) {
        for effect in &mut self.effects {
            effect.reset();
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl Default for EffectChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert decibels to linear amplitude
#[inline]
pub fn db_to_linear(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

/// Convert linear amplitude to decibels
#[inline]
pub fn linear_to_db(linear: f32) -> f32 {
    20.0 * linear.max(1e-10).log10()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gain() {
        let mut gain = Gain::new(6.0); // +6 dB
        let mut samples = vec![0.5, -0.5, 0.25];
        gain.process(&mut samples);

        // +6 dB ≈ 2x
        assert!((samples[0] - 1.0).abs() < 0.01);
        assert!((samples[1] + 1.0).abs() < 0.01);
    }

    #[test]
    fn test_db_conversion() {
        assert!((db_to_linear(0.0) - 1.0).abs() < 0.001);
        assert!((db_to_linear(6.0) - 2.0).abs() < 0.01);
        assert!((db_to_linear(-6.0) - 0.5).abs() < 0.01);

        assert!((linear_to_db(1.0) - 0.0).abs() < 0.001);
        assert!((linear_to_db(2.0) - 6.0).abs() < 0.1);
    }

    #[test]
    fn test_low_pass_filter() {
        let mut lpf = LowPassFilter::new(1000.0, 48000.0);
        let mut samples: Vec<f32> = (0..1000).map(|i| (i as f32 * 0.1).sin()).collect();

        lpf.process(&mut samples);

        // Output should be smoothed
        assert!(samples.iter().all(|&s| s.abs() < 1.1));
    }

    #[test]
    fn test_delay() {
        let mut delay = Delay::new(10.0, 48000.0);
        delay.mix = 1.0; // Full wet

        let mut samples = vec![1.0, 0.0, 0.0, 0.0, 0.0];
        delay.process(&mut samples);

        // First sample should be delayed (from buffer which is zero)
        assert_eq!(samples[0], 0.0);
    }

    #[test]
    fn test_effect_chain() {
        let mut chain = EffectChain::new();
        chain.add(Box::new(Gain::new(0.0)));
        chain.add(Box::new(Gain::new(0.0)));

        assert_eq!(chain.len(), 2);

        let mut samples = vec![0.5, -0.5];
        chain.process(&mut samples);

        // Unity gain, samples should be unchanged
        assert!((samples[0] - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_compressor() {
        let mut comp = Compressor::new(48000.0);
        comp.threshold_db = -20.0;
        comp.ratio = 4.0;

        let mut samples = vec![0.5, 0.5, 0.5, 0.5];
        comp.process(&mut samples);

        // Samples above threshold should be compressed
        assert!(samples.iter().all(|&s| s.abs() < 0.6));
    }

    #[test]
    fn test_noise_gate_opens_above_threshold() {
        let mut gate = NoiseGate::new(48000.0);
        gate.threshold_db = -40.0;

        // Loud signal (0.5 = -6dB, well above -40dB threshold)
        let mut samples = vec![0.5; 1000];
        gate.process(&mut samples);

        // Gate should be open, signal should pass (with attack time smoothing)
        // After enough samples, the gate should be fully open
        let last_samples: Vec<f32> = samples.iter().rev().take(100).cloned().collect();
        assert!(last_samples.iter().any(|&s| s > 0.3));
    }

    #[test]
    fn test_noise_gate_closes_below_threshold() {
        let mut gate = NoiseGate::new(48000.0);
        gate.threshold_db = -20.0; // -20dB = 0.1 linear

        // First, open the gate with a loud signal
        let mut loud = vec![0.5; 1000];
        gate.process(&mut loud);

        // Quiet signal (0.01 = -40dB, well below -20dB threshold)
        let mut quiet = vec![0.01; 5000];
        gate.process(&mut quiet);

        // After release time, gate should be mostly closed
        let last_sample = *quiet.last().unwrap();
        assert!(last_sample < 0.01, "Gate should attenuate quiet signal");
    }

    #[test]
    fn test_noise_gate_disabled() {
        let mut gate = NoiseGate::new(48000.0);
        gate.threshold_db = -20.0;
        gate.set_enabled(false);

        let mut samples = vec![0.001; 100]; // Very quiet
        let original = samples.clone();
        gate.process(&mut samples);

        // When disabled, samples should pass through unchanged
        assert_eq!(samples, original);
    }

    #[test]
    fn test_high_pass_filter() {
        let mut hpf = HighPassFilter::new(1000.0, 48000.0);

        // DC offset should be removed
        let mut samples = vec![1.0; 1000];
        hpf.process(&mut samples);

        // After settling, output should approach zero for DC input
        let last_samples: Vec<f32> = samples.iter().rev().take(100).cloned().collect();
        assert!(last_samples.iter().all(|&s| s.abs() < 0.1));
    }
}
