//! Tauri application library
//!
//! Provides IPC commands for signaling server, audio device, and streaming management.

mod audio;
mod signaling;
mod streaming;

use audio::AudioState;
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
            streaming::streaming_start,
            streaming::streaming_stop,
            streaming::streaming_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
