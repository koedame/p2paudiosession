//! Audio engine module
//!
//! Handles audio capture, playback, and local monitoring.

mod device;
mod engine;
mod error;

pub use device::{list_input_devices, list_output_devices, AudioDevice, DeviceId};
pub use engine::{AudioConfig, AudioEngine};
pub use error::AudioError;
