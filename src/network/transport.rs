//! UDP transport layer

use std::net::SocketAddr;
use std::sync::Arc;

use socket2::{Domain, Protocol, Socket, Type};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tracing::{debug, error, info, trace};

use crate::protocol::Packet;

use super::error::NetworkError;

/// UDP transport for sending and receiving packets
pub struct UdpTransport {
    socket: Arc<UdpSocket>,
    local_addr: SocketAddr,
}

impl UdpTransport {
    /// Bind to a local address with SO_REUSEADDR enabled
    pub async fn bind(addr: &str) -> Result<Self, NetworkError> {
        let parsed_addr: SocketAddr = addr.parse()?;

        // Create socket with socket2 for SO_REUSEADDR support
        let domain = if parsed_addr.is_ipv4() {
            Domain::IPV4
        } else {
            Domain::IPV6
        };
        let socket = Socket::new(domain, Type::DGRAM, Some(Protocol::UDP))?;

        // Enable SO_REUSEADDR to allow quick rebind after session leave
        socket.set_reuse_address(true)?;

        // Set non-blocking mode for async operation
        socket.set_nonblocking(true)?;

        // Bind to the address
        socket.bind(&parsed_addr.into())?;

        // Convert to Tokio UdpSocket
        let std_socket: std::net::UdpSocket = socket.into();
        let socket = UdpSocket::from_std(std_socket)?;
        let local_addr = socket.local_addr()?;

        info!(
            "UDP transport bound to {} (SO_REUSEADDR enabled)",
            local_addr
        );

        Ok(Self {
            socket: Arc::new(socket),
            local_addr,
        })
    }

    /// Get the local address
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Send a packet to a remote address
    pub async fn send_to(&self, packet: &Packet, addr: SocketAddr) -> Result<(), NetworkError> {
        let data = packet.to_bytes();
        self.socket.send_to(&data, addr).await?;
        trace!("Sent {} bytes to {}", data.len(), addr);
        Ok(())
    }

    /// Receive a packet (returns packet and sender address)
    pub async fn recv_from(&self) -> Result<(Packet, SocketAddr), NetworkError> {
        let mut buf = vec![0u8; 2048];
        let (len, addr) = self.socket.recv_from(&mut buf).await?;
        buf.truncate(len);

        let packet = Packet::from_bytes(&buf).ok_or(NetworkError::InvalidPacket)?;
        trace!("Received {} bytes from {}", len, addr);

        Ok((packet, addr))
    }

    /// Receive raw bytes (for connectivity probing without packet parsing)
    pub async fn recv_raw(&self) -> Result<(Vec<u8>, SocketAddr), NetworkError> {
        let mut buf = vec![0u8; 2048];
        let (len, addr) = self.socket.recv_from(&mut buf).await?;
        buf.truncate(len);
        trace!("Received {} raw bytes from {}", len, addr);
        Ok((buf, addr))
    }

    /// Start a receive loop that sends packets to a channel
    pub fn start_receive_loop(
        self: Arc<Self>,
    ) -> (
        mpsc::Receiver<(Packet, SocketAddr)>,
        tokio::task::JoinHandle<()>,
    ) {
        let (tx, rx) = mpsc::channel(1024);
        let socket = self.clone();

        let handle = tokio::spawn(async move {
            loop {
                match socket.recv_from().await {
                    Ok((packet, addr)) => {
                        if tx.send((packet, addr)).await.is_err() {
                            debug!("Receive channel closed, stopping receive loop");
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Receive error: {}", e);
                    }
                }
            }
        });

        (rx, handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_transport_bind() {
        let transport = UdpTransport::bind("127.0.0.1:0").await.unwrap();
        assert!(transport.local_addr().port() > 0);
    }

    #[tokio::test]
    async fn test_transport_send_receive() {
        let transport1 = UdpTransport::bind("127.0.0.1:0").await.unwrap();
        let transport2 = UdpTransport::bind("127.0.0.1:0").await.unwrap();

        let packet = Packet::audio(1, 100, vec![1, 2, 3, 4]);
        transport1
            .send_to(&packet, transport2.local_addr())
            .await
            .unwrap();

        let (received, from_addr) = transport2.recv_from().await.unwrap();
        assert_eq!(received.sequence, 1);
        assert_eq!(received.timestamp, 100);
        assert_eq!(received.payload, vec![1, 2, 3, 4]);
        assert_eq!(from_addr, transport1.local_addr());
    }

    /// Test SO_REUSEADDR allows rebinding to same port after drop
    #[tokio::test]
    async fn test_transport_port_reuse() {
        // First bind to get an assigned port
        let transport1 = UdpTransport::bind("127.0.0.1:0").await.unwrap();
        let port = transport1.local_addr().port();
        let addr = format!("127.0.0.1:{}", port);

        // Drop the first transport
        drop(transport1);

        // Should be able to immediately rebind to the same port (SO_REUSEADDR)
        let transport2 = UdpTransport::bind(&addr).await;
        assert!(
            transport2.is_ok(),
            "Should be able to rebind to same port with SO_REUSEADDR"
        );
        assert_eq!(transport2.unwrap().local_addr().port(), port);
    }

    /// Test multiple consecutive rebinds to same port
    #[tokio::test]
    async fn test_transport_repeated_port_reuse() {
        // First bind to get an assigned port
        let transport = UdpTransport::bind("127.0.0.1:0").await.unwrap();
        let port = transport.local_addr().port();
        let addr = format!("127.0.0.1:{}", port);
        drop(transport);

        // Rebind multiple times in succession
        for i in 0..5 {
            let transport = UdpTransport::bind(&addr).await;
            assert!(transport.is_ok(), "Rebind attempt {} should succeed", i + 1);
            drop(transport);
        }
    }
}
