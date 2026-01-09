//! Packet definitions for the jamjam protocol
//!
//! Packet format (12-byte header):
//! - version: 1 byte
//! - type: 1 byte
//! - sequence: 4 bytes (big-endian)
//! - timestamp: 4 bytes (big-endian, in samples)
//! - flags: 2 bytes

use serde::{Deserialize, Serialize};

/// Protocol version
pub const PROTOCOL_VERSION: u8 = 1;

/// Header size in bytes
pub const HEADER_SIZE: usize = 12;

/// Maximum payload size (MTU - IP header - UDP header - our header)
/// 1500 - 20 - 8 - 12 = 1460 bytes
#[allow(dead_code)]
pub const MAX_PAYLOAD_SIZE: usize = 1460;

/// Packet types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PacketType {
    /// Audio data
    Audio = 0x01,
    /// FEC redundancy data
    Fec = 0x02,
    /// Control message
    Control = 0x03,
    /// Keep-alive ping
    KeepAlive = 0x04,
}

impl TryFrom<u8> for PacketType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(PacketType::Audio),
            0x02 => Ok(PacketType::Fec),
            0x03 => Ok(PacketType::Control),
            0x04 => Ok(PacketType::KeepAlive),
            _ => Err(()),
        }
    }
}

/// Packet flags
#[derive(Debug, Clone, Copy, Default)]
pub struct PacketFlags {
    /// Packet is encrypted
    pub encrypted: bool,
    /// Packet contains FEC info
    pub has_fec: bool,
}

impl PacketFlags {
    pub fn to_u16(self) -> u16 {
        let mut flags = 0u16;
        if self.encrypted {
            flags |= 0x0001;
        }
        if self.has_fec {
            flags |= 0x0002;
        }
        flags
    }

    pub fn from_u16(value: u16) -> Self {
        Self {
            encrypted: (value & 0x0001) != 0,
            has_fec: (value & 0x0002) != 0,
        }
    }
}

/// A network packet
#[derive(Debug, Clone)]
pub struct Packet {
    pub version: u8,
    pub packet_type: PacketType,
    pub sequence: u32,
    pub timestamp: u32,
    pub flags: PacketFlags,
    pub payload: Vec<u8>,
}

impl Packet {
    /// Create a new audio packet
    pub fn audio(sequence: u32, timestamp: u32, payload: Vec<u8>) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            packet_type: PacketType::Audio,
            sequence,
            timestamp,
            flags: PacketFlags::default(),
            payload,
        }
    }

    /// Create a new keep-alive packet
    pub fn keep_alive(sequence: u32) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            packet_type: PacketType::KeepAlive,
            sequence,
            timestamp: 0,
            flags: PacketFlags::default(),
            payload: Vec::new(),
        }
    }

    /// Serialize the packet to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(HEADER_SIZE + self.payload.len());

        buf.push(self.version);
        buf.push(self.packet_type as u8);
        buf.extend_from_slice(&self.sequence.to_be_bytes());
        buf.extend_from_slice(&self.timestamp.to_be_bytes());
        buf.extend_from_slice(&self.flags.to_u16().to_be_bytes());
        buf.extend_from_slice(&self.payload);

        buf
    }

    /// Deserialize a packet from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < HEADER_SIZE {
            return None;
        }

        let version = data[0];
        if version != PROTOCOL_VERSION {
            return None;
        }

        let packet_type = PacketType::try_from(data[1]).ok()?;
        let sequence = u32::from_be_bytes([data[2], data[3], data[4], data[5]]);
        let timestamp = u32::from_be_bytes([data[6], data[7], data[8], data[9]]);
        let flags = PacketFlags::from_u16(u16::from_be_bytes([data[10], data[11]]));
        let payload = data[HEADER_SIZE..].to_vec();

        Some(Self {
            version,
            packet_type,
            sequence,
            timestamp,
            flags,
            payload,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_roundtrip() {
        let original = Packet::audio(42, 12345, vec![1, 2, 3, 4, 5]);
        let bytes = original.to_bytes();
        let decoded = Packet::from_bytes(&bytes).expect("Failed to decode packet");

        assert_eq!(decoded.version, original.version);
        assert_eq!(decoded.packet_type, original.packet_type);
        assert_eq!(decoded.sequence, original.sequence);
        assert_eq!(decoded.timestamp, original.timestamp);
        assert_eq!(decoded.payload, original.payload);
    }

    #[test]
    fn test_packet_type_conversion() {
        assert_eq!(PacketType::try_from(0x01), Ok(PacketType::Audio));
        assert_eq!(PacketType::try_from(0x02), Ok(PacketType::Fec));
        assert_eq!(PacketType::try_from(0x03), Ok(PacketType::Control));
        assert_eq!(PacketType::try_from(0x04), Ok(PacketType::KeepAlive));
        assert_eq!(PacketType::try_from(0xFF), Err(()));
    }

    #[test]
    fn test_flags_roundtrip() {
        let flags = PacketFlags {
            encrypted: true,
            has_fec: true,
        };
        let encoded = flags.to_u16();
        let decoded = PacketFlags::from_u16(encoded);

        assert_eq!(decoded.encrypted, flags.encrypted);
        assert_eq!(decoded.has_fec, flags.has_fec);
    }

    #[test]
    fn test_header_size() {
        let packet = Packet::audio(0, 0, vec![]);
        let bytes = packet.to_bytes();
        assert_eq!(bytes.len(), HEADER_SIZE);
    }

    #[test]
    fn test_invalid_packet_too_short() {
        let data = vec![0u8; HEADER_SIZE - 1];
        assert!(Packet::from_bytes(&data).is_none());
    }

    #[test]
    fn test_invalid_protocol_version() {
        let mut data = vec![0u8; HEADER_SIZE];
        data[0] = 99; // Invalid version
        assert!(Packet::from_bytes(&data).is_none());
    }
}
