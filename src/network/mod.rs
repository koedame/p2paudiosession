//! Network module for P2P communication
//!
//! Handles UDP transport, NAT traversal, signaling, FEC, and connection management.

mod connection;
mod error;
mod fec;
mod session;
mod signaling;
mod stun;
mod transport;

pub use connection::Connection;
pub use error::NetworkError;
pub use fec::{FecDecoder, FecEncoder, FecPacket, RecoveredPacket, FEC_GROUP_SIZE};
pub use session::{Session, SessionConfig};
pub use signaling::{
    PeerInfo, RoomInfo, SignalingClient, SignalingConnection, SignalingMessage, SignalingServer,
};
pub use stun::{StunClient, StunResult, DEFAULT_STUN_SERVERS};
pub use transport::UdpTransport;
