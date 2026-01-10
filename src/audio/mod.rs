//! Audio engine module
//!
//! Handles audio capture, playback, recording, metronome, and local monitoring.

mod device;
mod engine;
mod error;
mod metronome;
mod recording;

pub use device::{list_input_devices, list_output_devices, AudioDevice, DeviceId};
pub use engine::{AudioConfig, AudioEngine};
pub use error::AudioError;
pub use metronome::{Metronome, MetronomeConfig, MetronomeState, MetronomeSync};
pub use recording::{Recorder, RecordingInfo};
