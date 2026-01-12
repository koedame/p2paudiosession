//! Virtual audio device management
//!
//! Provides cross-platform virtual audio device setup for E2E testing.
//! - Linux: PipeWire null-audio-sink
//! - macOS: BlackHole
//! - Windows: VB-Audio Virtual Cable

use std::path::PathBuf;
use std::process::Command;
use tracing::{debug, info, warn};

use crate::node::Platform;

/// Virtual audio device configuration
#[derive(Debug, Clone)]
pub struct VirtualAudioConfig {
    /// Sample rate
    pub sample_rate: u32,
    /// Number of channels
    pub channels: u16,
    /// Device name prefix
    pub name_prefix: String,
}

impl Default for VirtualAudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            channels: 2,
            name_prefix: "jamjam-test".to_string(),
        }
    }
}

/// Virtual audio device manager
pub struct VirtualAudioManager {
    config: VirtualAudioConfig,
    platform: Platform,
    sink_created: bool,
    source_created: bool,
}

impl VirtualAudioManager {
    /// Create a new virtual audio manager
    pub fn new(config: VirtualAudioConfig) -> Self {
        Self {
            config,
            platform: Platform::current(),
            sink_created: false,
            source_created: false,
        }
    }

    /// Get the script path for the current platform
    #[allow(dead_code)]
    fn get_script_path(&self) -> PathBuf {
        let script_name = self.platform.virtual_audio_script();
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("scripts")
            .join(script_name)
    }

    /// Check if virtual audio is available on this system
    pub fn is_available(&self) -> bool {
        match self.platform {
            Platform::Linux => self.check_pipewire_available(),
            Platform::MacOS => self.check_blackhole_available(),
            Platform::Windows => self.check_vbcable_available(),
        }
    }

    /// Setup virtual audio devices
    pub fn setup(&mut self) -> Result<(), VirtualAudioError> {
        info!("Setting up virtual audio devices for {:?}", self.platform);

        match self.platform {
            Platform::Linux => self.setup_linux(),
            Platform::MacOS => self.setup_macos(),
            Platform::Windows => self.setup_windows(),
        }
    }

    /// Teardown virtual audio devices
    pub fn teardown(&mut self) -> Result<(), VirtualAudioError> {
        info!("Tearing down virtual audio devices");

        match self.platform {
            Platform::Linux => self.teardown_linux(),
            Platform::MacOS => Ok(()), // BlackHole doesn't need teardown
            Platform::Windows => Ok(()), // VB-Cable doesn't need teardown
        }
    }

    /// Get the sink device name (for audio output/playback)
    pub fn sink_name(&self) -> String {
        match self.platform {
            Platform::Linux => format!("{}-sink", self.config.name_prefix),
            Platform::MacOS => "BlackHole 2ch".to_string(),
            Platform::Windows => "CABLE Input".to_string(),
        }
    }

    /// Get the source device name (for audio input/capture)
    pub fn source_name(&self) -> String {
        match self.platform {
            Platform::Linux => format!("{}-source", self.config.name_prefix),
            Platform::MacOS => "BlackHole 2ch".to_string(),
            Platform::Windows => "CABLE Output".to_string(),
        }
    }

    // Linux (PipeWire) implementation
    fn check_pipewire_available(&self) -> bool {
        Command::new("pw-cli")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn setup_linux(&mut self) -> Result<(), VirtualAudioError> {
        // Check if PipeWire is running
        let status = Command::new("pgrep")
            .arg("-x")
            .arg("pipewire")
            .status()
            .map_err(|e| VirtualAudioError::SetupFailed(e.to_string()))?;

        if !status.success() {
            return Err(VirtualAudioError::NotAvailable(
                "PipeWire is not running. Start with: systemctl --user start pipewire".to_string(),
            ));
        }

        // Create virtual sink
        let sink_name = format!("{}-sink", self.config.name_prefix);
        let sink_cmd = format!(
            r#"{{
                factory.name = support.null-audio-sink
                node.name = "{}"
                node.description = "jamjam Test Sink"
                media.class = Audio/Sink
                audio.position = [ FL FR ]
                audio.rate = {}
            }}"#,
            sink_name, self.config.sample_rate
        );

        let output = Command::new("pw-cli")
            .args(["create-node", "adapter", &sink_cmd])
            .output()
            .map_err(|e| VirtualAudioError::SetupFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("already exists") {
                warn!("Failed to create sink: {}", stderr);
            }
        }
        self.sink_created = true;

        // Create virtual source
        let source_name = format!("{}-source", self.config.name_prefix);
        let source_cmd = format!(
            r#"{{
                factory.name = support.null-audio-sink
                node.name = "{}"
                node.description = "jamjam Test Source"
                media.class = Audio/Source
                audio.position = [ FL FR ]
                audio.rate = {}
            }}"#,
            source_name, self.config.sample_rate
        );

        let output = Command::new("pw-cli")
            .args(["create-node", "adapter", &source_cmd])
            .output()
            .map_err(|e| VirtualAudioError::SetupFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("already exists") {
                warn!("Failed to create source: {}", stderr);
            }
        }
        self.source_created = true;

        info!("Virtual audio devices created: sink={}, source={}", sink_name, source_name);
        Ok(())
    }

    fn teardown_linux(&mut self) -> Result<(), VirtualAudioError> {
        // Find and destroy sink
        if self.sink_created {
            let sink_name = format!("{}-sink", self.config.name_prefix);
            if let Ok(output) = Command::new("pw-cli")
                .args(["list-objects", "Node"])
                .output()
            {
                let _stdout = String::from_utf8_lossy(&output.stdout);
                // Parse output to find node ID and destroy it
                // This is a simplified implementation
                debug!("Looking for sink {} to destroy", sink_name);
            }
        }

        // Find and destroy source
        if self.source_created {
            let source_name = format!("{}-source", self.config.name_prefix);
            debug!("Looking for source {} to destroy", source_name);
        }

        self.sink_created = false;
        self.source_created = false;
        Ok(())
    }

    // macOS (BlackHole) implementation
    fn check_blackhole_available(&self) -> bool {
        Command::new("system_profiler")
            .args(["SPAudioDataType"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("BlackHole"))
            .unwrap_or(false)
    }

    fn setup_macos(&mut self) -> Result<(), VirtualAudioError> {
        if !self.check_blackhole_available() {
            return Err(VirtualAudioError::NotAvailable(
                "BlackHole is not installed. Install with: brew install blackhole-2ch".to_string(),
            ));
        }

        info!("BlackHole is available and ready to use");
        Ok(())
    }

    // Windows (VB-Cable) implementation
    fn check_vbcable_available(&self) -> bool {
        // Check for VB-Cable via registry or WMI
        // This is a simplified check
        #[cfg(target_os = "windows")]
        {
            Command::new("powershell")
                .args(["-Command", "Get-WmiObject Win32_SoundDevice | Where-Object { $_.Name -like '*CABLE*' }"])
                .output()
                .map(|o| !o.stdout.is_empty())
                .unwrap_or(false)
        }
        #[cfg(not(target_os = "windows"))]
        {
            false
        }
    }

    fn setup_windows(&mut self) -> Result<(), VirtualAudioError> {
        if !self.check_vbcable_available() {
            return Err(VirtualAudioError::NotAvailable(
                "VB-Cable is not installed. Download from: https://vb-audio.com/Cable/".to_string(),
            ));
        }

        info!("VB-Cable is available and ready to use");
        Ok(())
    }
}

impl Drop for VirtualAudioManager {
    fn drop(&mut self) {
        if let Err(e) = self.teardown() {
            warn!("Failed to teardown virtual audio: {}", e);
        }
    }
}

/// Audio injector that uses virtual devices
pub struct VirtualAudioInjector {
    manager: VirtualAudioManager,
}

impl VirtualAudioInjector {
    /// Create a new virtual audio injector
    pub fn new(config: VirtualAudioConfig) -> Result<Self, VirtualAudioError> {
        let mut manager = VirtualAudioManager::new(config);
        manager.setup()?;
        Ok(Self { manager })
    }

    /// Inject audio from a WAV file into the virtual sink
    pub fn inject_file(&self, wav_path: &std::path::Path) -> Result<(), VirtualAudioError> {
        info!("Injecting audio from {:?} to {}", wav_path, self.manager.sink_name());

        match self.manager.platform {
            Platform::Linux => self.inject_linux(wav_path),
            Platform::MacOS => self.inject_macos(wav_path),
            Platform::Windows => self.inject_windows(wav_path),
        }
    }

    fn inject_linux(&self, wav_path: &std::path::Path) -> Result<(), VirtualAudioError> {
        // Use pw-play or paplay to inject audio
        let output = Command::new("pw-play")
            .arg("--target")
            .arg(self.manager.sink_name())
            .arg(wav_path)
            .output();

        match output {
            Ok(o) if o.status.success() => Ok(()),
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                Err(VirtualAudioError::InjectionFailed(stderr.to_string()))
            }
            Err(e) => {
                // Try paplay as fallback
                let output = Command::new("paplay")
                    .arg("--device")
                    .arg(self.manager.sink_name())
                    .arg(wav_path)
                    .output()
                    .map_err(|e| VirtualAudioError::InjectionFailed(e.to_string()))?;

                if output.status.success() {
                    Ok(())
                } else {
                    Err(VirtualAudioError::InjectionFailed(e.to_string()))
                }
            }
        }
    }

    fn inject_macos(&self, _wav_path: &std::path::Path) -> Result<(), VirtualAudioError> {
        // Use afplay with BlackHole
        // Note: afplay doesn't support device selection directly
        // Would need to use a more complex solution like SwitchAudioSource
        Err(VirtualAudioError::NotImplemented(
            "macOS audio injection requires additional tooling".to_string(),
        ))
    }

    fn inject_windows(&self, _wav_path: &std::path::Path) -> Result<(), VirtualAudioError> {
        Err(VirtualAudioError::NotImplemented(
            "Windows audio injection requires additional tooling".to_string(),
        ))
    }
}

/// Audio capturer that uses virtual devices
pub struct VirtualAudioCapturer {
    manager: VirtualAudioManager,
    output_path: PathBuf,
}

impl VirtualAudioCapturer {
    /// Create a new virtual audio capturer
    pub fn new(config: VirtualAudioConfig, output_path: PathBuf) -> Result<Self, VirtualAudioError> {
        let mut manager = VirtualAudioManager::new(config);
        manager.setup()?;
        Ok(Self { manager, output_path })
    }

    /// Start capturing audio from the virtual source
    pub fn start_capture(&self, duration_sec: f32) -> Result<(), VirtualAudioError> {
        info!(
            "Capturing audio from {} for {}s to {:?}",
            self.manager.source_name(),
            duration_sec,
            self.output_path
        );

        match self.manager.platform {
            Platform::Linux => self.capture_linux(duration_sec),
            Platform::MacOS => self.capture_macos(duration_sec),
            Platform::Windows => self.capture_windows(duration_sec),
        }
    }

    fn capture_linux(&self, _duration_sec: f32) -> Result<(), VirtualAudioError> {
        // Use pw-record to capture audio
        let output = Command::new("pw-record")
            .arg("--target")
            .arg(self.manager.source_name())
            .arg("--format")
            .arg("f32")
            .arg("--rate")
            .arg(self.manager.config.sample_rate.to_string())
            .arg("--channels")
            .arg(self.manager.config.channels.to_string())
            .arg(&self.output_path)
            .output();

        match output {
            Ok(o) if o.status.success() => Ok(()),
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                Err(VirtualAudioError::CaptureFailed(stderr.to_string()))
            }
            Err(e) => Err(VirtualAudioError::CaptureFailed(e.to_string())),
        }
    }

    fn capture_macos(&self, _duration_sec: f32) -> Result<(), VirtualAudioError> {
        Err(VirtualAudioError::NotImplemented(
            "macOS audio capture requires additional tooling".to_string(),
        ))
    }

    fn capture_windows(&self, _duration_sec: f32) -> Result<(), VirtualAudioError> {
        Err(VirtualAudioError::NotImplemented(
            "Windows audio capture requires additional tooling".to_string(),
        ))
    }
}

/// Errors that can occur during virtual audio operations
#[derive(Debug, thiserror::Error)]
pub enum VirtualAudioError {
    #[error("Virtual audio not available: {0}")]
    NotAvailable(String),

    #[error("Setup failed: {0}")]
    SetupFailed(String),

    #[error("Audio injection failed: {0}")]
    InjectionFailed(String),

    #[error("Audio capture failed: {0}")]
    CaptureFailed(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_audio_config_default() {
        let config = VirtualAudioConfig::default();
        assert_eq!(config.sample_rate, 48000);
        assert_eq!(config.channels, 2);
        assert_eq!(config.name_prefix, "jamjam-test");
    }

    #[test]
    fn test_device_names() {
        let config = VirtualAudioConfig::default();
        let manager = VirtualAudioManager::new(config);

        // Device names should be non-empty
        assert!(!manager.sink_name().is_empty());
        assert!(!manager.source_name().is_empty());
    }

    #[test]
    fn test_script_path_exists() {
        let config = VirtualAudioConfig::default();
        let manager = VirtualAudioManager::new(config);
        let script_path = manager.get_script_path();

        // Script should exist
        assert!(script_path.exists(), "Script not found: {:?}", script_path);
    }
}
