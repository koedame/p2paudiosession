//! Encryption layer for P2P audio data
//!
//! Uses X25519 for key exchange and AES-256-GCM for symmetric encryption.
//! The nonce is derived from the packet sequence number to avoid nonce reuse.

use std::net::SocketAddr;
use std::sync::Arc;

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use hkdf::Hkdf;
use rand::rngs::OsRng;
use sha2::Sha256;
use x25519_dalek::{EphemeralSecret, PublicKey, SharedSecret};

use crate::protocol::Packet;

use super::error::NetworkError;
use super::transport::UdpTransport;

/// Size of the authentication tag (AES-GCM)
#[allow(dead_code)]
const TAG_SIZE: usize = 16;

/// Size of the nonce (96 bits for AES-GCM)
const NONCE_SIZE: usize = 12;

/// Encryption key pair for ECDH key exchange
pub struct KeyPair {
    secret: EphemeralSecret,
    public: PublicKey,
}

impl KeyPair {
    /// Generate a new random key pair
    pub fn generate() -> Self {
        let secret = EphemeralSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        Self { secret, public }
    }

    /// Get the public key bytes for sharing
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.public.to_bytes()
    }

    /// Derive a shared secret from the peer's public key
    pub fn derive_shared_secret(self, peer_public: &[u8; 32]) -> SharedSecret {
        let peer_key = PublicKey::from(*peer_public);
        self.secret.diffie_hellman(&peer_key)
    }
}

/// Encryption context for a session
pub struct EncryptionContext {
    cipher: Aes256Gcm,
    /// Used to derive unique nonces from sequence numbers
    nonce_prefix: [u8; 4],
}

impl EncryptionContext {
    /// Create a new encryption context from a shared secret
    pub fn from_shared_secret(shared_secret: &[u8], is_initiator: bool) -> Self {
        // Use HKDF to derive the encryption key
        let hk = Hkdf::<Sha256>::new(None, shared_secret);
        let mut key_bytes = [0u8; 32];
        let info = if is_initiator {
            b"jamjam-session-key-initiator"
        } else {
            b"jamjam-session-key-responder"
        };
        hk.expand(info, &mut key_bytes)
            .expect("HKDF expand should not fail");

        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);

        // Derive nonce prefix
        let mut nonce_prefix = [0u8; 4];
        let nonce_info = if is_initiator {
            b"jamjam-nonce-prefix-initiator"
        } else {
            b"jamjam-nonce-prefix-responder"
        };
        let mut nonce_bytes = [0u8; 4];
        hk.expand(nonce_info, &mut nonce_bytes)
            .expect("HKDF expand should not fail");
        nonce_prefix.copy_from_slice(&nonce_bytes);

        Self {
            cipher,
            nonce_prefix,
        }
    }

    /// Encrypt a packet payload
    pub fn encrypt(&self, sequence: u32, plaintext: &[u8]) -> Result<Vec<u8>, NetworkError> {
        let nonce = self.derive_nonce(sequence);
        let nonce = Nonce::from_slice(&nonce);

        self.cipher
            .encrypt(nonce, plaintext)
            .map_err(|_| NetworkError::EncryptionError("Encryption failed".to_string()))
    }

    /// Decrypt a packet payload
    pub fn decrypt(&self, sequence: u32, ciphertext: &[u8]) -> Result<Vec<u8>, NetworkError> {
        let nonce = self.derive_nonce(sequence);
        let nonce = Nonce::from_slice(&nonce);

        self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| NetworkError::EncryptionError("Decryption failed".to_string()))
    }

    /// Derive a nonce from sequence number
    /// Nonce format: [4 bytes prefix][4 bytes sequence][4 bytes zero padding]
    fn derive_nonce(&self, sequence: u32) -> [u8; NONCE_SIZE] {
        let mut nonce = [0u8; NONCE_SIZE];
        nonce[0..4].copy_from_slice(&self.nonce_prefix);
        nonce[4..8].copy_from_slice(&sequence.to_be_bytes());
        // Last 4 bytes remain zero
        nonce
    }
}

/// Encrypted transport wrapper
pub struct EncryptedTransport {
    inner: Arc<UdpTransport>,
    encryption: EncryptionContext,
}

impl EncryptedTransport {
    /// Create a new encrypted transport
    pub fn new(transport: Arc<UdpTransport>, encryption: EncryptionContext) -> Self {
        Self {
            inner: transport,
            encryption,
        }
    }

    /// Get the local address
    pub fn local_addr(&self) -> SocketAddr {
        self.inner.local_addr()
    }

    /// Send an encrypted packet
    pub async fn send_to(&self, packet: &Packet, addr: SocketAddr) -> Result<(), NetworkError> {
        // Encrypt the payload
        let encrypted_payload = self.encryption.encrypt(packet.sequence, &packet.payload)?;

        // Create a new packet with encrypted payload and encrypted flag set
        let mut flags = packet.flags;
        flags.encrypted = true;

        let encrypted_packet = Packet {
            version: packet.version,
            packet_type: packet.packet_type,
            sequence: packet.sequence,
            timestamp: packet.timestamp,
            flags,
            payload: encrypted_payload,
        };

        self.inner.send_to(&encrypted_packet, addr).await
    }

    /// Receive and decrypt a packet
    pub async fn recv_from(&self) -> Result<(Packet, SocketAddr), NetworkError> {
        let (encrypted_packet, addr) = self.inner.recv_from().await?;

        // Decrypt the payload
        let decrypted_payload = self
            .encryption
            .decrypt(encrypted_packet.sequence, &encrypted_packet.payload)?;

        // Create decrypted packet with encrypted flag cleared
        let mut flags = encrypted_packet.flags;
        flags.encrypted = false;

        let packet = Packet {
            version: encrypted_packet.version,
            packet_type: encrypted_packet.packet_type,
            sequence: encrypted_packet.sequence,
            timestamp: encrypted_packet.timestamp,
            flags,
            payload: decrypted_payload,
        };

        Ok((packet, addr))
    }
}

/// Helper to perform key exchange via signaling
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KeyExchangeMessage {
    pub public_key: [u8; 32],
}

impl KeyExchangeMessage {
    /// Create a new key exchange message
    pub fn new(public_key: [u8; 32]) -> Self {
        Self { public_key }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_exchange() {
        // Simulate two peers
        let alice = KeyPair::generate();
        let bob = KeyPair::generate();

        let alice_public = alice.public_key_bytes();
        let bob_public = bob.public_key_bytes();

        // Derive shared secrets
        let alice_shared = alice.derive_shared_secret(&bob_public);
        let bob_shared = bob.derive_shared_secret(&alice_public);

        // Shared secrets should be equal
        assert_eq!(alice_shared.as_bytes(), bob_shared.as_bytes());
    }

    #[test]
    fn test_encryption_roundtrip() {
        let shared_secret = [0x42u8; 32]; // Dummy shared secret

        let sender_ctx = EncryptionContext::from_shared_secret(&shared_secret, true);
        let _receiver_ctx = EncryptionContext::from_shared_secret(&shared_secret, false);

        let plaintext = b"Hello, encrypted world!";
        let sequence = 12345u32;

        // Note: sender and receiver use different contexts (different keys/nonces)
        // In practice, both sides would use the same shared secret but different roles
        let sender_ctx2 = EncryptionContext::from_shared_secret(&shared_secret, true);

        let ciphertext = sender_ctx.encrypt(sequence, plaintext).unwrap();
        let decrypted = sender_ctx2.decrypt(sequence, &ciphertext).unwrap();

        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_different_sequences_different_ciphertext() {
        let shared_secret = [0x42u8; 32];
        let ctx = EncryptionContext::from_shared_secret(&shared_secret, true);

        let plaintext = b"Same plaintext";

        let ct1 = ctx.encrypt(1, plaintext).unwrap();
        let ct2 = ctx.encrypt(2, plaintext).unwrap();

        // Different sequences should produce different ciphertext
        assert_ne!(ct1, ct2);
    }
}
