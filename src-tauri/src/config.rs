//! Configuration persistence module
//!
//! Provides TOML-based configuration file management for the jamjam application.
//! Configuration is stored in platform-specific directories:
//! - Linux: ~/.config/jamjam/config.toml
//! - Windows: %APPDATA%\jamjam\config.toml
//! - macOS: ~/Library/Application Support/jamjam/config.toml

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

/// Application name used for configuration directory
const APP_NAME: &str = "jamjam";

/// Default buffer size in samples (64 samples @ 48kHz = 1.33ms)
const DEFAULT_BUFFER_SIZE: u32 = 64;

/// Application configuration
///
/// Contains all persistent settings for the jamjam application.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    /// Selected input device ID (None = system default)
    #[serde(default)]
    pub input_device_id: Option<String>,

    /// Selected output device ID (None = system default)
    #[serde(default)]
    pub output_device_id: Option<String>,

    /// Audio buffer size in samples. Valid values: 32, 64, 128, 256
    #[serde(default = "default_buffer_size")]
    pub buffer_size: u32,

    /// Custom signaling server URL (None = use default server)
    #[serde(default)]
    pub signaling_server_url: Option<String>,
}

fn default_buffer_size() -> u32 {
    DEFAULT_BUFFER_SIZE
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            input_device_id: None,
            output_device_id: None,
            buffer_size: DEFAULT_BUFFER_SIZE,
            signaling_server_url: None,
        }
    }
}

impl AppConfig {
    /// Validate the configuration values
    ///
    /// Returns an error message if any value is invalid.
    pub fn validate(&self) -> Result<(), String> {
        // Validate buffer size
        if ![32, 64, 128, 256].contains(&self.buffer_size) {
            return Err(format!(
                "Invalid buffer size: {}. Valid values are 32, 64, 128, 256",
                self.buffer_size
            ));
        }

        // Validate signaling server URL if provided
        if let Some(ref url) = self.signaling_server_url {
            if !url.starts_with("ws://") && !url.starts_with("wss://") {
                return Err(format!(
                    "Invalid signaling server URL: {}. Must start with ws:// or wss://",
                    url
                ));
            }
        }

        Ok(())
    }
}

/// State for configuration management
pub struct ConfigState {
    config: Mutex<AppConfig>,
}

impl ConfigState {
    /// Create a new ConfigState, loading existing config or using defaults
    pub fn new() -> Self {
        let config = load_config().unwrap_or_default();
        Self {
            config: Mutex::new(config),
        }
    }

    /// Get a clone of the current configuration
    pub fn get(&self) -> Result<AppConfig, String> {
        self.config
            .lock()
            .map(|guard| guard.clone())
            .map_err(|e| format!("Failed to lock config: {}", e))
    }

    /// Update the configuration and save to disk
    pub fn update(&self, new_config: AppConfig) -> Result<(), String> {
        new_config.validate()?;

        let mut config = self
            .config
            .lock()
            .map_err(|e| format!("Failed to lock config: {}", e))?;

        *config = new_config.clone();
        drop(config);

        save_config(&new_config)
    }
}

impl Default for ConfigState {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the configuration directory path
///
/// Returns None if the configuration directory cannot be determined.
fn get_config_dir() -> Option<PathBuf> {
    ProjectDirs::from("", "", APP_NAME).map(|dirs| dirs.config_dir().to_path_buf())
}

/// Get the configuration file path
///
/// Returns None if the configuration directory cannot be determined.
fn get_config_path() -> Option<PathBuf> {
    get_config_dir().map(|dir| dir.join("config.toml"))
}

/// Load configuration from the config file
///
/// Returns the loaded configuration, or an error if loading fails.
/// If the file doesn't exist, returns an error (use unwrap_or_default for fallback).
pub fn load_config() -> Result<AppConfig, String> {
    let path = get_config_path().ok_or("Could not determine config path")?;

    if !path.exists() {
        return Err("Config file does not exist".to_string());
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read config file at {:?}: {}", path, e))?;

    let config: AppConfig =
        toml::from_str(&content).map_err(|e| format!("Failed to parse config file: {}", e))?;

    // Validate the loaded config
    config.validate()?;

    Ok(config)
}

/// Save configuration to the config file
///
/// Creates the config directory if it doesn't exist.
pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let config_dir = get_config_dir().ok_or("Could not determine config directory")?;
    let config_path = config_dir.join("config.toml");

    // Create config directory if it doesn't exist
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)
            .map_err(|e| format!("Failed to create config directory {:?}: {}", config_dir, e))?;
    }

    // Serialize config to TOML
    let content =
        toml::to_string_pretty(config).map_err(|e| format!("Failed to serialize config: {}", e))?;

    // Write to file
    fs::write(&config_path, content)
        .map_err(|e| format!("Failed to write config file {:?}: {}", config_path, e))?;

    Ok(())
}

// =============================================================================
// Tauri Commands
// =============================================================================

/// Load configuration from disk
///
/// Returns the current configuration (from file or defaults if file doesn't exist).
#[tauri::command]
pub fn config_load(state: tauri::State<'_, ConfigState>) -> Result<AppConfig, String> {
    state.get()
}

/// Save configuration to disk
///
/// Validates and saves the provided configuration.
#[tauri::command]
pub fn config_save(config: AppConfig, state: tauri::State<'_, ConfigState>) -> Result<(), String> {
    state.update(config)
}

/// Get the signaling server URL from configuration
///
/// Returns None if using the default server.
#[tauri::command]
pub fn config_get_server_url(
    state: tauri::State<'_, ConfigState>,
) -> Result<Option<String>, String> {
    let config = state.get()?;
    Ok(config.signaling_server_url)
}

/// Set the signaling server URL in configuration
///
/// Pass None to use the default server.
#[tauri::command]
pub fn config_set_server_url(
    url: Option<String>,
    state: tauri::State<'_, ConfigState>,
) -> Result<(), String> {
    let mut config = state.get()?;
    config.signaling_server_url = url;
    state.update(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.input_device_id, None);
        assert_eq!(config.output_device_id, None);
        assert_eq!(config.buffer_size, 64);
        assert_eq!(config.signaling_server_url, None);
    }

    #[test]
    fn test_config_validation_valid() {
        let config = AppConfig {
            input_device_id: Some("device1".to_string()),
            output_device_id: Some("device2".to_string()),
            buffer_size: 64,
            signaling_server_url: Some("wss://example.com".to_string()),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_invalid_buffer_size() {
        let config = AppConfig {
            buffer_size: 100, // Invalid
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_invalid_url() {
        let config = AppConfig {
            signaling_server_url: Some("http://example.com".to_string()), // Invalid, should be ws/wss
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let config = AppConfig {
            input_device_id: Some("input-device".to_string()),
            output_device_id: Some("output-device".to_string()),
            buffer_size: 128,
            signaling_server_url: Some("wss://server.example.com".to_string()),
        };

        let toml_str = toml::to_string(&config).unwrap();
        let parsed: AppConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(config, parsed);
    }

    #[test]
    fn test_config_deserialization_with_defaults() {
        // Test that missing fields get default values
        let toml_str = r#"
            input_device_id = "some-device"
        "#;

        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.input_device_id, Some("some-device".to_string()));
        assert_eq!(config.output_device_id, None);
        assert_eq!(config.buffer_size, 64); // Default
        assert_eq!(config.signaling_server_url, None);
    }
}
