//! Audio engine module
//!
//! Handles audio capture, playback, recording, metronome, effects, plugins, and local monitoring.

mod device;
mod effects;
mod engine;
mod error;
mod metronome;
mod plugin;
mod recording;

pub use device::{list_input_devices, list_output_devices, AudioDevice, DeviceId};
pub use effects::{
    Compressor, Delay, Effect, EffectChain, Gain, HighPassFilter, LowPassFilter, NoiseGate,
    db_to_linear, linear_to_db,
};
pub use engine::{AudioConfig, AudioEngine};
pub use error::AudioError;
pub use metronome::{Metronome, MetronomeConfig, MetronomeState, MetronomeSync};
pub use plugin::{AudioPlugin, PluginFormat, PluginHost, PluginInfo, PluginParameter, PluginScanner};
pub use recording::{Recorder, RecordingInfo};
