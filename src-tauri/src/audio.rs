//! Audio IPC commands for Tauri
//!
//! Provides commands to list and select audio devices.

use std::sync::Mutex;

use serde::Serialize;

use jamjam::audio::{list_input_devices, list_output_devices};

/// Audio state managed by Tauri
///
/// Note: AudioEngine is not stored here because cpal's Stream is not Send+Sync.
/// This state only tracks the selected device IDs and buffer settings.
/// Actual audio engine initialization will be done separately when needed.
pub struct AudioState {
    current_input_id: Mutex<Option<String>>,
    current_output_id: Mutex<Option<String>>,
    /// Buffer size (frame_size) in samples. Valid values: 32, 64, 128, 256
    buffer_size: Mutex<u32>,
}

impl AudioState {
    pub fn new() -> Self {
        Self {
            current_input_id: Mutex::new(None),
            current_output_id: Mutex::new(None),
            buffer_size: Mutex::new(64), // Default: 64 samples @ 48kHz = 1.33ms
        }
    }
}

impl Default for AudioState {
    fn default() -> Self {
        Self::new()
    }
}

/// Audio device information for IPC
#[derive(Debug, Clone, Serialize)]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
    pub supported_sample_rates: Vec<u32>,
    pub supported_channels: Vec<u16>,
    pub is_default: bool,
    pub is_asio: bool,
}

/// Current device selection
#[derive(Debug, Clone, Serialize)]
pub struct CurrentDevices {
    pub input_device_id: Option<String>,
    pub output_device_id: Option<String>,
}

/// List available input (microphone) devices
#[tauri::command]
pub fn audio_list_input_devices() -> Result<Vec<AudioDeviceInfo>, String> {
    let devices = list_input_devices().map_err(|e| e.to_string())?;

    Ok(devices
        .into_iter()
        .map(|d| AudioDeviceInfo {
            id: d.id.0,
            name: d.name,
            supported_sample_rates: d.supported_sample_rates,
            supported_channels: d.supported_channels,
            is_default: d.is_default,
            is_asio: d.is_asio,
        })
        .collect())
}

/// List available output (speaker) devices
#[tauri::command]
pub fn audio_list_output_devices() -> Result<Vec<AudioDeviceInfo>, String> {
    let devices = list_output_devices().map_err(|e| e.to_string())?;

    Ok(devices
        .into_iter()
        .map(|d| AudioDeviceInfo {
            id: d.id.0,
            name: d.name,
            supported_sample_rates: d.supported_sample_rates,
            supported_channels: d.supported_channels,
            is_default: d.is_default,
            is_asio: d.is_asio,
        })
        .collect())
}

/// Set the input device
///
/// Note: This currently only stores the device ID selection.
/// Actual audio engine device switching will be implemented when
/// the audio pipeline is integrated.
#[tauri::command]
pub fn audio_set_input_device(
    device_id: Option<String>,
    state: tauri::State<'_, AudioState>,
) -> Result<(), String> {
    let mut current = state.current_input_id.lock().map_err(|e| e.to_string())?;
    *current = device_id;

    Ok(())
}

/// Set the output device
///
/// Note: This currently only stores the device ID selection.
/// Actual audio engine device switching will be implemented when
/// the audio pipeline is integrated.
#[tauri::command]
pub fn audio_set_output_device(
    device_id: Option<String>,
    state: tauri::State<'_, AudioState>,
) -> Result<(), String> {
    let mut current = state.current_output_id.lock().map_err(|e| e.to_string())?;
    *current = device_id;

    Ok(())
}

/// Get current device selection
#[tauri::command]
pub fn audio_get_current_devices(
    state: tauri::State<'_, AudioState>,
) -> Result<CurrentDevices, String> {
    let input = state
        .current_input_id
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    let output = state
        .current_output_id
        .lock()
        .map_err(|e| e.to_string())?
        .clone();

    Ok(CurrentDevices {
        input_device_id: input,
        output_device_id: output,
    })
}

/// Get current buffer size (frame_size in samples)
#[tauri::command]
pub fn audio_get_buffer_size(state: tauri::State<'_, AudioState>) -> Result<u32, String> {
    let size = state.buffer_size.lock().map_err(|e| e.to_string())?;
    Ok(*size)
}

/// Set buffer size (frame_size in samples)
///
/// Valid values: 32, 64, 128, 256
/// Lower values = less latency but may cause audio crackling
/// Higher values = more stable but higher latency
#[tauri::command]
pub fn audio_set_buffer_size(size: u32, state: tauri::State<'_, AudioState>) -> Result<(), String> {
    // Validate buffer size
    if ![32, 64, 128, 256].contains(&size) {
        return Err(format!(
            "Invalid buffer size: {}. Valid values are 32, 64, 128, 256",
            size
        ));
    }

    let mut current = state.buffer_size.lock().map_err(|e| e.to_string())?;
    *current = size;

    Ok(())
}
