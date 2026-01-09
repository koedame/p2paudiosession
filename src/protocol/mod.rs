//! Network protocol definitions
//!
//! Defines the packet format for audio data transmission.

mod packet;

pub use packet::{Packet, PacketType, HEADER_SIZE, PROTOCOL_VERSION};
