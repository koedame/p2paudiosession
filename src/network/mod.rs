//! Network module for P2P communication
//!
//! Handles UDP transport and connection management.

mod connection;
mod error;
mod transport;

pub use connection::Connection;
pub use error::NetworkError;
pub use transport::UdpTransport;
