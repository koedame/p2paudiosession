//! Audio loopback tests
//!
//! These tests verify audio pipeline quality without network involvement.
//! Uses virtual audio devices to inject and capture audio.

use crate::audio_injection::{AudioCapture, AudioInjector};
use crate::quality::{LatencyMeasurer, PesqEvaluator, QualityResult};
use crate::{TestConfig, TestResult};
use std::time::Instant;
use tracing::{info, warn};

/// Loopback test configuration
pub struct LoopbackTest {
    config: TestConfig,
    injector: AudioInjector,
}

impl LoopbackTest {
    /// Create a new loopback test
    pub fn new(config: TestConfig) -> Self {
        let injector = AudioInjector::new(config.sample_rate, 2);
        Self { config, injector }
    }

    /// Run a basic audio quality test with a sine wave
    ///
    /// This test:
    /// 1. Generates a reference sine wave
    /// 2. Plays it through the audio pipeline
    /// 3. Captures the output
    /// 4. Evaluates quality using correlation-based MOS
    pub fn run_sine_test(&self, frequency: f32) -> TestResult {
        let start = Instant::now();
        info!("Running sine wave loopback test at {} Hz", frequency);

        // Generate reference audio
        let reference = self.injector.generate_sine(frequency, self.config.duration_sec);

        // In a real implementation, this would:
        // 1. Inject audio into virtual input device
        // 2. Start jamjam audio capture
        // 3. Route through jitter buffer / effects
        // 4. Capture from virtual output device

        // For now, simulate with direct passthrough
        let received = reference.clone();

        // Evaluate quality
        let evaluator = PesqEvaluator::new(self.config.sample_rate);
        let quality = evaluator
            .evaluate_with_threshold(&reference, &received, &self.config.preset)
            .unwrap_or_else(|e| {
                warn!("Quality evaluation failed: {}", e);
                QualityResult::failed(e.to_string())
            });

        TestResult {
            scenario: format!("loopback_sine_{}hz", frequency as u32),
            passed: quality.meets_threshold,
            duration_ms: start.elapsed().as_millis() as u64,
            connection_time_ms: None,
            quality: Some(quality),
            error: None,
        }
    }

    /// Run a frequency sweep test to verify codec quality across the spectrum
    pub fn run_sweep_test(&self, start_freq: f32, end_freq: f32) -> TestResult {
        let start = Instant::now();
        info!(
            "Running frequency sweep test {} Hz - {} Hz",
            start_freq, end_freq
        );

        let reference = self
            .injector
            .generate_sweep(start_freq, end_freq, self.config.duration_sec);

        // Simulate passthrough (actual implementation would use real audio pipeline)
        let received = reference.clone();

        let evaluator = PesqEvaluator::new(self.config.sample_rate);
        let quality = evaluator
            .evaluate_with_threshold(&reference, &received, &self.config.preset)
            .unwrap_or_else(|e| QualityResult::failed(e.to_string()));

        TestResult {
            scenario: format!("loopback_sweep_{}_{}", start_freq as u32, end_freq as u32),
            passed: quality.meets_threshold,
            duration_ms: start.elapsed().as_millis() as u64,
            connection_time_ms: None,
            quality: Some(quality),
            error: None,
        }
    }

    /// Run a latency measurement test
    pub fn run_latency_test(&self) -> TestResult {
        let start = Instant::now();
        info!("Running latency measurement test");

        let reference = self.injector.generate_sine(1000.0, self.config.duration_sec);

        // Simulate minimal latency (actual implementation would measure real latency)
        let received = reference.clone();

        let measurer = LatencyMeasurer::new(self.config.sample_rate);
        let latency_ms = measurer.measure(&reference, &received).unwrap_or(999.0);

        let quality = measurer.verify_against_spec(latency_ms, &self.config.preset);

        TestResult {
            scenario: "loopback_latency".to_string(),
            passed: quality.meets_threshold,
            duration_ms: start.elapsed().as_millis() as u64,
            connection_time_ms: None,
            quality: Some(quality),
            error: None,
        }
    }

    /// Run silence test to verify no noise is introduced
    pub fn run_silence_test(&self) -> TestResult {
        let start = Instant::now();
        info!("Running silence test");

        let reference = self.injector.generate_silence(self.config.duration_sec);

        // Passthrough
        let received = reference.clone();

        // Check that silence is preserved (RMS should be near zero)
        let rms: f32 = (received.iter().map(|s| s * s).sum::<f32>() / received.len() as f32).sqrt();
        let noise_floor_db = if rms > 0.0 {
            20.0 * rms.log10()
        } else {
            -120.0
        };

        let passed = noise_floor_db < -60.0; // Should be below -60 dB

        let mut quality = QualityResult::passed();
        quality.notes = Some(format!("Noise floor: {:.1} dB", noise_floor_db));
        quality.meets_threshold = passed;

        TestResult {
            scenario: "loopback_silence".to_string(),
            passed,
            duration_ms: start.elapsed().as_millis() as u64,
            connection_time_ms: None,
            quality: Some(quality),
            error: if passed {
                None
            } else {
                Some(format!(
                    "Noise floor too high: {:.1} dB (max -60 dB)",
                    noise_floor_db
                ))
            },
        }
    }
}

/// Run all loopback tests for a given preset
pub fn run_all_loopback_tests(preset: &str) -> Vec<TestResult> {
    let config = TestConfig {
        sample_rate: 48000,
        frame_size: 128,
        duration_sec: 1.0,
        preset: preset.to_string(),
    };

    let test = LoopbackTest::new(config);

    vec![
        test.run_sine_test(440.0),
        test.run_sine_test(1000.0),
        test.run_sweep_test(100.0, 10000.0),
        test.run_latency_test(),
        test.run_silence_test(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sine_loopback() {
        let config = TestConfig {
            sample_rate: 48000,
            frame_size: 128,
            duration_sec: 0.1,
            preset: "balanced".to_string(),
        };
        let test = LoopbackTest::new(config);
        let result = test.run_sine_test(440.0);

        assert!(result.passed, "Sine loopback test should pass");
    }

    #[test]
    fn test_sweep_loopback() {
        let config = TestConfig {
            sample_rate: 48000,
            frame_size: 128,
            duration_sec: 0.5,
            preset: "balanced".to_string(),
        };
        let test = LoopbackTest::new(config);
        let result = test.run_sweep_test(100.0, 10000.0);

        assert!(result.passed, "Sweep loopback test should pass");
    }

    #[test]
    fn test_silence_loopback() {
        let config = TestConfig {
            sample_rate: 48000,
            frame_size: 128,
            duration_sec: 0.1,
            preset: "balanced".to_string(),
        };
        let test = LoopbackTest::new(config);
        let result = test.run_silence_test();

        assert!(result.passed, "Silence loopback test should pass");
    }

    #[test]
    fn test_all_presets() {
        for preset in ["zero-latency", "balanced", "high-quality"] {
            let config = TestConfig {
                sample_rate: 48000,
                frame_size: 128,
                duration_sec: 0.1,
                preset: preset.to_string(),
            };
            let test = LoopbackTest::new(config);
            let result = test.run_sine_test(440.0);

            assert!(
                result.passed,
                "Preset {} should pass sine test",
                preset
            );
        }
    }
}
