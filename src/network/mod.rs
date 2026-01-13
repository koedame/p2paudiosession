//! Network module for P2P communication
//!
//! Handles UDP transport, NAT traversal, signaling, FEC, encryption, and connection management.

mod connection;
mod encryption;
mod error;
mod fec;
mod jitter_buffer;
mod sequence_tracker;
mod session;
mod signaling;
mod stun;
mod transport;

pub use connection::{Connection, ConnectionState};
pub use encryption::{EncryptedTransport, EncryptionContext, KeyExchangeMessage, KeyPair};
pub use error::NetworkError;
pub use fec::{FecDecoder, FecEncoder, FecPacket, RecoveredPacket, FEC_GROUP_SIZE};
pub use jitter_buffer::{
    JitterBuffer, JitterBufferConfig, JitterBufferMode, JitterBufferResult, JitterBufferStats,
};
pub use sequence_tracker::SequenceTracker;
pub use session::{Session, SessionConfig};
pub use signaling::{
    PeerInfo, RoomInfo, SignalingClient, SignalingConnection, SignalingMessage, SignalingServer,
    MAX_PEERS_PER_ROOM,
};
pub use stun::{StunClient, StunResult, DEFAULT_STUN_SERVERS};
pub use transport::UdpTransport;
