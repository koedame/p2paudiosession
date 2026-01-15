//! Tauri application library
//!
//! Provides IPC commands for signaling server, audio device, and streaming management.

mod audio;
mod config;
mod signaling;
mod streaming;

use audio::AudioState;
use config::ConfigState;
use signaling::SignalingState;
use streaming::StreamingState;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(SignalingState::new())
        .manage(AudioState::new())
        .manage(StreamingState::new())
        .manage(ConfigState::new())
        .invoke_handler(tauri::generate_handler![
            greet,
            signaling::signaling_connect,
            signaling::signaling_disconnect,
            signaling::signaling_list_rooms,
            signaling::signaling_join_room,
            signaling::signaling_leave_room,
            signaling::signaling_create_room,
            audio::audio_list_input_devices,
            audio::audio_list_output_devices,
            audio::audio_set_input_device,
            audio::audio_set_output_device,
            audio::audio_get_current_devices,
            audio::audio_get_buffer_size,
            audio::audio_set_buffer_size,
            streaming::streaming_start,
            streaming::streaming_stop,
            streaming::streaming_status,
            streaming::streaming_set_input_device,
            streaming::streaming_set_output_device,
            config::config_load,
            config::config_save,
            config::config_get_server_url,
            config::config_set_server_url,
            config::config_list_presets,
            config::config_get_preset,
            config::config_set_preset,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
