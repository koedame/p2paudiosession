//! Network error types

use thiserror::Error;

/// Errors that can occur in the network subsystem
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Connection timeout")]
    ConnectionTimeout,

    #[error("Connection refused")]
    ConnectionRefused,

    #[error("Already connected")]
    AlreadyConnected,

    #[error("Not connected")]
    NotConnected,

    #[error("Send buffer full")]
    SendBufferFull,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid packet")]
    InvalidPacket,

    #[error("Address parse error: {0}")]
    AddrParse(#[from] std::net::AddrParseError),

    #[error("STUN failed: {0}")]
    StunFailed(String),

    #[error("Signaling error: {0}")]
    SignalingError(String),

    #[error("Session full")]
    SessionFull,

    #[error("Peer not found: {0}")]
    PeerNotFound(String),

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Key exchange failed: {0}")]
    KeyExchangeFailed(String),
}
