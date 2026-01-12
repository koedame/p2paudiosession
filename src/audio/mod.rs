//! Audio engine module
//!
//! Handles audio capture, playback, recording, metronome, effects, plugins, and local monitoring.

mod codec;
mod device;
mod effects;
mod engine;
mod error;
mod metronome;
mod plc;
mod plugin;
mod recording;

pub use codec::{
    create_codec, AudioCodec, CodecConfig, CodecError, CodecType, OpusCodec, PcmCodec,
};
pub use device::{list_input_devices, list_output_devices, AudioDevice, DeviceId};
pub use effects::{
    db_to_linear, linear_to_db, Compressor, Delay, Effect, EffectChain, Gain, HighPassFilter,
    LowPassFilter, NoiseGate,
};
pub use engine::{
    AudioBuffer, AudioConfig, AudioEngine, BitDepth, CaptureConfig, PlaybackConfig,
    SharedPlaybackProducer,
};
pub use error::AudioError;
pub use metronome::{Metronome, MetronomeConfig, MetronomeState, MetronomeSync};
pub use plc::PcmPlc;
pub use plugin::{
    AudioPlugin, ClapPlugin, ClapPluginLoader, PluginFormat, PluginHost, PluginInfo,
    PluginParameter, PluginScanner,
};
pub use recording::{Recorder, RecordingInfo};
