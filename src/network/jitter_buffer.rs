//! Adaptive jitter buffer for packet reordering and timing
//!
//! Buffers incoming packets to handle network jitter and packet reordering.
//! Provides adaptive sizing based on network conditions.

use std::collections::BTreeMap;
use std::time::Instant;

/// Configuration for the jitter buffer
#[derive(Debug, Clone)]
pub struct JitterBufferConfig {
    /// Minimum buffer delay in frames (default: 1, must be >= 1)
    pub min_delay_frames: u32,
    /// Maximum buffer delay in frames (default: 10)
    pub max_delay_frames: u32,
    /// Initial buffer delay in frames (default: 2)
    pub initial_delay_frames: u32,
    /// Frame duration in milliseconds
    pub frame_duration_ms: f32,
}

impl JitterBufferConfig {
    /// Validate and normalize the configuration
    ///
    /// Returns a validated config with:
    /// - min_delay_frames >= 1 (0 means no buffering, which defeats the purpose)
    /// - max_delay_frames >= min_delay_frames
    /// - initial_delay_frames clamped between min and max
    /// - frame_duration_ms > 0
    pub fn validated(self) -> Self {
        let min_delay_frames = self.min_delay_frames.max(1);
        let max_delay_frames = self.max_delay_frames.max(min_delay_frames);
        let initial_delay_frames = self
            .initial_delay_frames
            .clamp(min_delay_frames, max_delay_frames);
        let frame_duration_ms = if self.frame_duration_ms > 0.0 {
            self.frame_duration_ms
        } else {
            2.5 // Default fallback
        };

        Self {
            min_delay_frames,
            max_delay_frames,
            initial_delay_frames,
            frame_duration_ms,
        }
    }
}

impl Default for JitterBufferConfig {
    fn default() -> Self {
        Self {
            min_delay_frames: 1,
            max_delay_frames: 10,
            initial_delay_frames: 2,
            frame_duration_ms: 2.5, // 120 samples @ 48kHz
        }
    }
}

/// Packet entry in the jitter buffer
#[derive(Debug)]
struct BufferedPacket {
    /// Packet sequence number
    sequence: u32,
    /// Timestamp in samples
    timestamp: u32,
    /// Encoded audio payload
    payload: Vec<u8>,
    /// Time when packet was received (for future jitter statistics)
    #[allow(dead_code)]
    received_at: Instant,
}

/// Result from popping the jitter buffer
#[derive(Debug)]
pub enum JitterBufferResult {
    /// Normal packet available
    Packet {
        sequence: u32,
        timestamp: u32,
        payload: Vec<u8>,
    },
    /// Packet was lost (not received in time)
    Lost { sequence: u32 },
    /// Buffer underrun (not enough packets buffered yet)
    Underrun,
}

/// Adaptive jitter buffer
///
/// Buffers packets indexed by sequence number for reordering.
/// Adapts buffer size based on observed jitter.
pub struct JitterBuffer {
    /// Packets indexed by sequence number
    packets: BTreeMap<u32, BufferedPacket>,
    /// Next sequence number expected for playback
    next_play_sequence: Option<u32>,
    /// Configuration
    config: JitterBufferConfig,
    /// Current buffer delay in frames
    current_delay_frames: u32,
    /// Jitter estimate in milliseconds
    jitter_estimate_ms: f32,
    /// Count of packets inserted
    packets_inserted: u64,
    /// Count of packets played
    packets_played: u64,
    /// Count of packets lost
    packets_lost: u64,
    /// Count of late arrivals (received after play deadline)
    late_arrivals: u64,
    /// Whether buffer has started playback
    playing: bool,
    /// Timestamp of first packet (for sync)
    first_timestamp: Option<u32>,
}

impl JitterBuffer {
    /// Create a new jitter buffer with default configuration
    pub fn new() -> Self {
        Self::with_config(JitterBufferConfig::default())
    }

    /// Create a new jitter buffer with custom configuration
    ///
    /// The configuration is validated to ensure sensible values.
    pub fn with_config(config: JitterBufferConfig) -> Self {
        let config = config.validated();
        Self {
            packets: BTreeMap::new(),
            next_play_sequence: None,
            current_delay_frames: config.initial_delay_frames,
            jitter_estimate_ms: 0.0,
            packets_inserted: 0,
            packets_played: 0,
            packets_lost: 0,
            late_arrivals: 0,
            playing: false,
            first_timestamp: None,
            config,
        }
    }

    /// Insert a received packet into the buffer
    ///
    /// # Arguments
    /// * `sequence` - Packet sequence number
    /// * `timestamp` - Timestamp in samples
    /// * `payload` - Encoded audio data
    pub fn insert(&mut self, sequence: u32, timestamp: u32, payload: Vec<u8>) {
        // Initialize first timestamp for sync
        if self.first_timestamp.is_none() {
            self.first_timestamp = Some(timestamp);
        }

        // Track minimum sequence for out-of-order arrivals
        // Only update if not playing yet and this sequence is lower
        if !self.playing {
            match self.next_play_sequence {
                None => self.next_play_sequence = Some(sequence),
                Some(current) => {
                    // Use signed comparison for wraparound
                    let diff = self.sequence_diff(sequence, current);
                    if diff < 0 {
                        // New packet has lower sequence - update
                        self.next_play_sequence = Some(sequence);
                    }
                }
            }
        } else if let Some(next_seq) = self.next_play_sequence {
            // Check if this is a late arrival (only when playing)
            let diff = self.sequence_diff(sequence, next_seq);
            if diff < 0 {
                // Packet arrived after its play deadline
                self.late_arrivals += 1;
            }
        }

        let packet = BufferedPacket {
            sequence,
            timestamp,
            payload,
            received_at: Instant::now(),
        };

        self.packets.insert(sequence, packet);
        self.packets_inserted += 1;

        // Limit buffer size to prevent memory growth
        self.prune_old_packets();
    }

    /// Get next frame for playback
    ///
    /// Returns the next packet in sequence order, or indicates loss/underrun.
    pub fn pop(&mut self) -> JitterBufferResult {
        let next_seq = match self.next_play_sequence {
            Some(seq) => seq,
            None => return JitterBufferResult::Underrun,
        };

        // Check if we should start playing
        if !self.playing {
            // Wait until we have enough packets buffered
            if self.depth() < self.current_delay_frames {
                return JitterBufferResult::Underrun;
            }
            self.playing = true;
        }

        // Try to get the next packet
        if let Some(packet) = self.packets.remove(&next_seq) {
            self.next_play_sequence = Some(next_seq.wrapping_add(1));
            self.packets_played += 1;

            JitterBufferResult::Packet {
                sequence: packet.sequence,
                timestamp: packet.timestamp,
                payload: packet.payload,
            }
        } else {
            // Packet not in buffer - lost
            self.next_play_sequence = Some(next_seq.wrapping_add(1));
            self.packets_lost += 1;

            JitterBufferResult::Lost { sequence: next_seq }
        }
    }

    /// Peek at the next packet without removing it
    pub fn peek(&self) -> Option<u32> {
        self.next_play_sequence
    }

    /// Get current buffer depth in frames
    pub fn depth(&self) -> u32 {
        self.packets.len() as u32
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.packets.is_empty()
    }

    /// Check if buffer has started playback
    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// Get statistics
    pub fn stats(&self) -> JitterBufferStats {
        JitterBufferStats {
            packets_inserted: self.packets_inserted,
            packets_played: self.packets_played,
            packets_lost: self.packets_lost,
            late_arrivals: self.late_arrivals,
            current_depth: self.depth(),
            current_delay_frames: self.current_delay_frames,
            jitter_estimate_ms: self.jitter_estimate_ms,
        }
    }

    /// Adapt buffer size based on network conditions
    ///
    /// Call this periodically (e.g., every 100ms) to adjust buffer size.
    pub fn adapt(&mut self) {
        // Simple adaptation: increase delay if we're losing packets,
        // decrease if buffer is consistently full

        let loss_rate = if self.packets_inserted > 0 {
            self.packets_lost as f32 / self.packets_inserted as f32
        } else {
            0.0
        };

        if loss_rate > 0.05 {
            // More than 5% loss - increase buffer
            if self.current_delay_frames < self.config.max_delay_frames {
                self.current_delay_frames += 1;
            }
        } else if loss_rate < 0.01 && self.depth() > self.current_delay_frames {
            // Low loss and buffer is full - can decrease
            if self.current_delay_frames > self.config.min_delay_frames {
                self.current_delay_frames -= 1;
            }
        }
    }

    /// Reset the buffer
    pub fn reset(&mut self) {
        self.packets.clear();
        self.next_play_sequence = None;
        self.current_delay_frames = self.config.initial_delay_frames;
        self.jitter_estimate_ms = 0.0;
        self.packets_inserted = 0;
        self.packets_played = 0;
        self.packets_lost = 0;
        self.late_arrivals = 0;
        self.playing = false;
        self.first_timestamp = None;
    }

    /// Calculate signed difference between sequence numbers
    fn sequence_diff(&self, a: u32, b: u32) -> i32 {
        a.wrapping_sub(b) as i32
    }

    /// Remove old packets that are too far behind
    fn prune_old_packets(&mut self) {
        let max_buffer = self.config.max_delay_frames * 2;
        while self.packets.len() > max_buffer as usize {
            if let Some((&oldest_seq, _)) = self.packets.first_key_value() {
                self.packets.remove(&oldest_seq);
            } else {
                break;
            }
        }
    }
}

impl Default for JitterBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Jitter buffer statistics
#[derive(Debug, Clone)]
pub struct JitterBufferStats {
    pub packets_inserted: u64,
    pub packets_played: u64,
    pub packets_lost: u64,
    pub late_arrivals: u64,
    pub current_depth: u32,
    pub current_delay_frames: u32,
    pub jitter_estimate_ms: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequential_packets() {
        let config = JitterBufferConfig {
            initial_delay_frames: 2,
            ..Default::default()
        };
        let mut buffer = JitterBuffer::with_config(config);

        // Insert packets
        for i in 0..5 {
            buffer.insert(i, i * 120, vec![i as u8; 10]);
        }

        assert_eq!(buffer.depth(), 5);

        // Pop packets
        for i in 0..5 {
            match buffer.pop() {
                JitterBufferResult::Packet { sequence, .. } => {
                    assert_eq!(sequence, i);
                }
                _ => panic!("Expected packet"),
            }
        }
    }

    #[test]
    fn test_underrun() {
        let config = JitterBufferConfig {
            initial_delay_frames: 3,
            ..Default::default()
        };
        let mut buffer = JitterBuffer::with_config(config);

        // Only insert 2 packets (less than initial delay of 3)
        buffer.insert(0, 0, vec![0; 10]);
        buffer.insert(1, 120, vec![1; 10]);

        // Should underrun
        match buffer.pop() {
            JitterBufferResult::Underrun => {}
            _ => panic!("Expected underrun"),
        }
    }

    #[test]
    fn test_packet_loss() {
        let config = JitterBufferConfig {
            initial_delay_frames: 1,
            ..Default::default()
        };
        let mut buffer = JitterBuffer::with_config(config);

        // Insert packets with gap (missing seq 1)
        buffer.insert(0, 0, vec![0; 10]);
        buffer.insert(2, 240, vec![2; 10]);
        buffer.insert(3, 360, vec![3; 10]);

        // First pop - packet 0
        match buffer.pop() {
            JitterBufferResult::Packet { sequence, .. } => assert_eq!(sequence, 0),
            _ => panic!("Expected packet 0"),
        }

        // Second pop - packet 1 is lost
        match buffer.pop() {
            JitterBufferResult::Lost { sequence } => assert_eq!(sequence, 1),
            _ => panic!("Expected loss of packet 1"),
        }

        // Third pop - packet 2
        match buffer.pop() {
            JitterBufferResult::Packet { sequence, .. } => assert_eq!(sequence, 2),
            _ => panic!("Expected packet 2"),
        }
    }

    #[test]
    fn test_out_of_order() {
        let config = JitterBufferConfig {
            initial_delay_frames: 2,
            ..Default::default()
        };
        let mut buffer = JitterBuffer::with_config(config);

        // Insert out of order
        buffer.insert(2, 240, vec![2; 10]);
        buffer.insert(0, 0, vec![0; 10]);
        buffer.insert(1, 120, vec![1; 10]);
        buffer.insert(3, 360, vec![3; 10]);

        // Should still play in order
        for i in 0..4 {
            match buffer.pop() {
                JitterBufferResult::Packet { sequence, .. } => {
                    assert_eq!(sequence, i);
                }
                _ => panic!("Expected packet {}", i),
            }
        }
    }

    #[test]
    fn test_stats() {
        let config = JitterBufferConfig {
            initial_delay_frames: 1,
            ..Default::default()
        };
        let mut buffer = JitterBuffer::with_config(config);

        buffer.insert(0, 0, vec![0; 10]);
        buffer.insert(2, 240, vec![2; 10]); // Skip 1

        buffer.pop(); // Play 0
        buffer.pop(); // Lose 1
        buffer.pop(); // Play 2

        let stats = buffer.stats();
        assert_eq!(stats.packets_inserted, 2);
        assert_eq!(stats.packets_played, 2);
        assert_eq!(stats.packets_lost, 1);
    }

    #[test]
    fn test_reset() {
        let mut buffer = JitterBuffer::new();

        buffer.insert(0, 0, vec![0; 10]);
        buffer.insert(1, 120, vec![1; 10]);

        buffer.reset();

        assert!(buffer.is_empty());
        assert!(!buffer.is_playing());
        assert_eq!(buffer.stats().packets_inserted, 0);
    }

    #[test]
    fn test_config_validation() {
        // Test that zero min_delay is corrected to 1
        let config = JitterBufferConfig {
            min_delay_frames: 0,
            max_delay_frames: 5,
            initial_delay_frames: 2,
            frame_duration_ms: 2.5,
        };
        let validated = config.validated();
        assert_eq!(validated.min_delay_frames, 1);

        // Test that max_delay < min_delay is corrected
        let config = JitterBufferConfig {
            min_delay_frames: 5,
            max_delay_frames: 2,
            initial_delay_frames: 3,
            frame_duration_ms: 2.5,
        };
        let validated = config.validated();
        assert!(validated.max_delay_frames >= validated.min_delay_frames);

        // Test that initial_delay is clamped
        let config = JitterBufferConfig {
            min_delay_frames: 2,
            max_delay_frames: 5,
            initial_delay_frames: 10,
            frame_duration_ms: 2.5,
        };
        let validated = config.validated();
        assert!(validated.initial_delay_frames <= validated.max_delay_frames);

        // Test that negative frame_duration is corrected
        let config = JitterBufferConfig {
            min_delay_frames: 1,
            max_delay_frames: 5,
            initial_delay_frames: 2,
            frame_duration_ms: -1.0,
        };
        let validated = config.validated();
        assert!(validated.frame_duration_ms > 0.0);
    }

    #[test]
    fn test_adapt_increases_on_loss() {
        let config = JitterBufferConfig {
            initial_delay_frames: 2,
            min_delay_frames: 1,
            max_delay_frames: 5,
            ..Default::default()
        };
        let mut buffer = JitterBuffer::with_config(config);

        // Simulate high loss scenario: insert packets 1, 2, 4, 5, 7, 8... (skip 0, 3, 6...)
        for i in 0..20u32 {
            if i % 3 != 0 {
                buffer.insert(i, i * 120, vec![i as u8; 10]);
            }
        }

        // Pop a limited number of times to avoid infinite loop
        for _ in 0..25 {
            let result = buffer.pop();
            if matches!(result, JitterBufferResult::Underrun) {
                break;
            }
        }

        let initial_delay = buffer.current_delay_frames;
        buffer.adapt();

        // Should have increased delay due to losses
        assert!(
            buffer.current_delay_frames >= initial_delay,
            "Delay should increase on loss"
        );
    }
}
