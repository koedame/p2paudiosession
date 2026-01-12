//! E2E Test Infrastructure for jamjam
//!
//! This crate provides end-to-end testing capabilities for the jamjam
//! P2P audio communication application.
//!
//! ## Test Layers
//!
//! - **Loopback tests**: Test audio path without real devices (virtual audio)
//! - **Network tests**: Test P2P communication on localhost
//! - **Remote tests**: Test multi-node scenarios across machines
//!
//! ## Features
//!
//! - `loopback`: Enable virtual audio loopback tests
//! - `network-local`: Enable localhost network tests
//! - `remote`: Enable remote multi-node tests
//! - `full`: Enable all test features

pub mod audio_injection;
pub mod node;
pub mod orchestrator;
pub mod quality;
pub mod scenarios;
pub mod virtual_audio;

// Re-exports for convenience
pub use audio_injection::AudioInjector;
pub use node::{Platform, TestNode};
pub use orchestrator::TestOrchestrator;
pub use quality::{LatencyMeasurer, PesqEvaluator, QualityResult};
pub use virtual_audio::{VirtualAudioConfig, VirtualAudioManager};

/// Test configuration
#[derive(Debug, Clone)]
pub struct TestConfig {
    /// Sample rate for audio tests
    pub sample_rate: u32,
    /// Frame size in samples
    pub frame_size: u32,
    /// Test duration in seconds
    pub duration_sec: f32,
    /// Preset to test
    pub preset: String,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            frame_size: 128,
            duration_sec: 10.0,
            preset: "balanced".to_string(),
        }
    }
}

/// Result of an E2E test scenario
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TestResult {
    /// Test scenario name
    pub scenario: String,
    /// Whether the test passed
    pub passed: bool,
    /// Test duration in milliseconds
    pub duration_ms: u64,
    /// Connection establishment time in milliseconds
    pub connection_time_ms: Option<u64>,
    /// Audio quality metrics
    pub quality: Option<QualityResult>,
    /// Error message if failed
    pub error: Option<String>,
}

impl TestResult {
    /// Create a passed result
    pub fn passed(scenario: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            scenario: scenario.into(),
            passed: true,
            duration_ms,
            connection_time_ms: None,
            quality: None,
            error: None,
        }
    }

    /// Create a failed result
    pub fn failed(scenario: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            scenario: scenario.into(),
            passed: false,
            duration_ms: 0,
            connection_time_ms: None,
            quality: None,
            error: Some(error.into()),
        }
    }
}
