//! jamjam - Low-latency P2P audio communication for musicians
//!
//! This library provides the core functionality for real-time audio
//! streaming between musicians over a network.

pub mod audio;
pub mod network;
pub mod protocol;

pub use audio::{AudioConfig, AudioEngine};
pub use network::Connection;
pub use protocol::Packet;
