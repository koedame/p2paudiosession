//! STUN client for NAT traversal
//!
//! Implements RFC 5389 STUN (Session Traversal Utilities for NAT)
//! to discover public IP address and port mapping.

use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::time::{timeout, Duration};
use tracing::{debug, info, warn};

use super::error::NetworkError;

/// STUN message types
const BINDING_REQUEST: u16 = 0x0001;
const BINDING_RESPONSE: u16 = 0x0101;

/// STUN attribute types
const MAPPED_ADDRESS: u16 = 0x0001;
const XOR_MAPPED_ADDRESS: u16 = 0x0020;

/// STUN magic cookie (RFC 5389)
const MAGIC_COOKIE: u32 = 0x2112A442;

/// Default STUN servers
pub const DEFAULT_STUN_SERVERS: &[&str] = &[
    "stun.l.google.com:19302",
    "stun1.l.google.com:19302",
    "stun2.l.google.com:19302",
    "stun.cloudflare.com:3478",
];

/// Result of a STUN binding request
#[derive(Debug, Clone)]
pub struct StunResult {
    /// Public address as seen by the STUN server
    pub mapped_address: SocketAddr,
    /// STUN server that was used
    pub server: String,
}

/// STUN client for discovering public address
pub struct StunClient {
    socket: UdpSocket,
    timeout_ms: u64,
}

impl StunClient {
    /// Create a new STUN client bound to the given socket
    pub fn new(socket: UdpSocket) -> Self {
        Self {
            socket,
            timeout_ms: 3000,
        }
    }

    /// Create a new STUN client with custom timeout
    pub fn with_timeout(socket: UdpSocket, timeout_ms: u64) -> Self {
        Self { socket, timeout_ms }
    }

    /// Discover public address using default STUN servers
    pub async fn discover_public_address(&self) -> Result<StunResult, NetworkError> {
        for server in DEFAULT_STUN_SERVERS {
            match self.binding_request(server).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    warn!("STUN request to {} failed: {}", server, e);
                    continue;
                }
            }
        }
        Err(NetworkError::StunFailed(
            "All STUN servers failed".to_string(),
        ))
    }

    /// Send a binding request to a specific STUN server
    pub async fn binding_request(&self, server: &str) -> Result<StunResult, NetworkError> {
        let server_addr: SocketAddr = tokio::net::lookup_host(server)
            .await
            .map_err(|e| NetworkError::StunFailed(format!("DNS lookup failed: {}", e)))?
            .next()
            .ok_or_else(|| NetworkError::StunFailed("No address found".to_string()))?;

        debug!("Sending STUN binding request to {}", server_addr);

        // Build STUN binding request
        let transaction_id: [u8; 12] = rand::random();
        let request = build_binding_request(&transaction_id);

        // Send request
        self.socket
            .send_to(&request, server_addr)
            .await
            .map_err(|e| NetworkError::StunFailed(format!("Send failed: {}", e)))?;

        // Wait for response
        let mut buf = [0u8; 576]; // Minimum STUN message size
        let (len, _) = timeout(Duration::from_millis(self.timeout_ms), self.socket.recv_from(&mut buf))
            .await
            .map_err(|_| NetworkError::StunFailed("Timeout".to_string()))?
            .map_err(|e| NetworkError::StunFailed(format!("Receive failed: {}", e)))?;

        // Parse response
        let mapped_address = parse_binding_response(&buf[..len], &transaction_id)?;

        info!("STUN discovered public address: {}", mapped_address);

        Ok(StunResult {
            mapped_address,
            server: server.to_string(),
        })
    }
}

/// Build a STUN binding request message
fn build_binding_request(transaction_id: &[u8; 12]) -> Vec<u8> {
    let mut msg = Vec::with_capacity(20);

    // Message type: Binding Request
    msg.extend_from_slice(&BINDING_REQUEST.to_be_bytes());

    // Message length (0 for binding request with no attributes)
    msg.extend_from_slice(&0u16.to_be_bytes());

    // Magic cookie
    msg.extend_from_slice(&MAGIC_COOKIE.to_be_bytes());

    // Transaction ID (12 bytes)
    msg.extend_from_slice(transaction_id);

    msg
}

/// Parse a STUN binding response
fn parse_binding_response(data: &[u8], expected_txn_id: &[u8; 12]) -> Result<SocketAddr, NetworkError> {
    if data.len() < 20 {
        return Err(NetworkError::StunFailed("Response too short".to_string()));
    }

    // Check message type
    let msg_type = u16::from_be_bytes([data[0], data[1]]);
    if msg_type != BINDING_RESPONSE {
        return Err(NetworkError::StunFailed(format!(
            "Unexpected message type: 0x{:04x}",
            msg_type
        )));
    }

    // Check magic cookie
    let cookie = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    if cookie != MAGIC_COOKIE {
        return Err(NetworkError::StunFailed("Invalid magic cookie".to_string()));
    }

    // Check transaction ID
    if &data[8..20] != expected_txn_id {
        return Err(NetworkError::StunFailed(
            "Transaction ID mismatch".to_string(),
        ));
    }

    // Parse message length
    let msg_len = u16::from_be_bytes([data[2], data[3]]) as usize;
    if data.len() < 20 + msg_len {
        return Err(NetworkError::StunFailed(
            "Message truncated".to_string(),
        ));
    }

    // Parse attributes
    let mut offset = 20;
    while offset + 4 <= 20 + msg_len {
        let attr_type = u16::from_be_bytes([data[offset], data[offset + 1]]);
        let attr_len = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;

        if offset + 4 + attr_len > data.len() {
            break;
        }

        let attr_data = &data[offset + 4..offset + 4 + attr_len];

        match attr_type {
            XOR_MAPPED_ADDRESS => {
                return parse_xor_mapped_address(attr_data);
            }
            MAPPED_ADDRESS => {
                return parse_mapped_address(attr_data);
            }
            _ => {}
        }

        // Align to 4-byte boundary
        offset += 4 + ((attr_len + 3) & !3);
    }

    Err(NetworkError::StunFailed(
        "No mapped address in response".to_string(),
    ))
}

/// Parse XOR-MAPPED-ADDRESS attribute
fn parse_xor_mapped_address(data: &[u8]) -> Result<SocketAddr, NetworkError> {
    if data.len() < 8 {
        return Err(NetworkError::StunFailed(
            "XOR-MAPPED-ADDRESS too short".to_string(),
        ));
    }

    let family = data[1];
    let xor_port = u16::from_be_bytes([data[2], data[3]]);
    let port = xor_port ^ ((MAGIC_COOKIE >> 16) as u16);

    match family {
        0x01 => {
            // IPv4
            let xor_addr = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
            let addr = xor_addr ^ MAGIC_COOKIE;
            let ip = std::net::Ipv4Addr::from(addr);
            Ok(SocketAddr::new(ip.into(), port))
        }
        0x02 => {
            // IPv6
            if data.len() < 20 {
                return Err(NetworkError::StunFailed(
                    "XOR-MAPPED-ADDRESS IPv6 too short".to_string(),
                ));
            }
            // For IPv6, XOR with magic cookie + transaction ID
            // Simplified: just return error for now
            Err(NetworkError::StunFailed(
                "IPv6 not yet supported".to_string(),
            ))
        }
        _ => Err(NetworkError::StunFailed(format!(
            "Unknown address family: {}",
            family
        ))),
    }
}

/// Parse MAPPED-ADDRESS attribute (legacy, non-XOR)
fn parse_mapped_address(data: &[u8]) -> Result<SocketAddr, NetworkError> {
    if data.len() < 8 {
        return Err(NetworkError::StunFailed(
            "MAPPED-ADDRESS too short".to_string(),
        ));
    }

    let family = data[1];
    let port = u16::from_be_bytes([data[2], data[3]]);

    match family {
        0x01 => {
            // IPv4
            let ip = std::net::Ipv4Addr::new(data[4], data[5], data[6], data[7]);
            Ok(SocketAddr::new(ip.into(), port))
        }
        0x02 => {
            // IPv6
            Err(NetworkError::StunFailed(
                "IPv6 not yet supported".to_string(),
            ))
        }
        _ => Err(NetworkError::StunFailed(format!(
            "Unknown address family: {}",
            family
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_binding_request() {
        let txn_id = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let request = build_binding_request(&txn_id);

        assert_eq!(request.len(), 20);
        // Check message type
        assert_eq!(request[0], 0x00);
        assert_eq!(request[1], 0x01);
        // Check message length
        assert_eq!(request[2], 0x00);
        assert_eq!(request[3], 0x00);
        // Check magic cookie
        assert_eq!(request[4], 0x21);
        assert_eq!(request[5], 0x12);
        assert_eq!(request[6], 0xA4);
        assert_eq!(request[7], 0x42);
        // Check transaction ID
        assert_eq!(&request[8..20], &txn_id);
    }

    #[test]
    fn test_parse_xor_mapped_address_ipv4() {
        // XOR-MAPPED-ADDRESS for 192.168.1.100:5000
        // XOR port: 5000 (0x1388) ^ 0x2112 = 0x329A
        // XOR addr: 192.168.1.100 (0xC0A80164) ^ 0x2112A442 = 0xE1BAA526
        let data = [
            0x00, 0x01, // Reserved + Family (IPv4)
            0x32, 0x9A, // XOR'd port
            0xE1, 0xBA, 0xA5, 0x26, // XOR'd address
        ];

        let result = parse_xor_mapped_address(&data).unwrap();
        assert_eq!(result.port(), 5000);
        assert_eq!(result.ip().to_string(), "192.168.1.100");
    }
}
