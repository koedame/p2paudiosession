//! Sequence number tracking for packet loss detection
//!
//! Tracks received sequence numbers to detect lost packets and calculate
//! packet loss statistics.

/// Tracks sequence numbers to detect lost packets
///
/// Uses a sliding window approach to handle out-of-order packets
/// and sequence number wraparound.
pub struct SequenceTracker {
    /// Last received sequence number
    last_sequence: Option<u32>,
    /// Highest sequence number seen
    highest_sequence: u32,
    /// Bitmap of recently received sequences (for out-of-order detection)
    /// Bit N represents whether (highest_sequence - N) was received
    received_bitmap: u64,
    /// Total packets received
    packets_received: u64,
    /// Total packets lost
    packets_lost: u64,
    /// Window size for out-of-order detection
    window_size: u32,
}

impl SequenceTracker {
    /// Create a new sequence tracker
    pub fn new() -> Self {
        Self {
            last_sequence: None,
            highest_sequence: 0,
            received_bitmap: 0,
            packets_received: 0,
            packets_lost: 0,
            window_size: 64,
        }
    }

    /// Record a received packet sequence number
    ///
    /// Returns a list of sequence numbers that are considered lost
    /// (gaps between last received and current).
    pub fn record(&mut self, sequence: u32) -> Vec<u32> {
        self.packets_received += 1;

        // First packet
        if self.last_sequence.is_none() {
            self.last_sequence = Some(sequence);
            self.highest_sequence = sequence;
            self.received_bitmap = 1; // Mark current as received
            return Vec::new();
        }

        let mut lost_sequences = Vec::new();

        // Calculate difference handling wraparound
        let diff = self.sequence_diff(sequence, self.highest_sequence);

        if diff > 0 {
            // New packet ahead of highest
            if diff <= self.window_size as i64 {
                // Within window - check for gaps
                for i in 1..diff as u32 {
                    let missed_seq = self.highest_sequence.wrapping_add(i);
                    lost_sequences.push(missed_seq);
                    self.packets_lost += 1;
                }

                // Shift bitmap and mark current as received
                if diff < 64 {
                    self.received_bitmap <<= diff;
                } else {
                    self.received_bitmap = 0;
                }
                self.received_bitmap |= 1;
            } else {
                // Large jump - reset tracking
                self.received_bitmap = 1;
            }

            self.highest_sequence = sequence;
        } else if diff < 0 && diff > -(self.window_size as i64) {
            // Out-of-order packet within window
            let offset = (-diff) as u32;
            if offset < 64 {
                let mask = 1u64 << offset;
                if self.received_bitmap & mask == 0 {
                    // Was marked as lost, now received (late arrival)
                    self.received_bitmap |= mask;
                    if self.packets_lost > 0 {
                        self.packets_lost -= 1;
                    }
                    // Remove from lost list if we just reported it
                    lost_sequences.retain(|&s| s != sequence);
                }
                // else: duplicate packet, ignore
            }
        }
        // else: very old packet or duplicate, ignore

        self.last_sequence = Some(sequence);

        lost_sequences
    }

    /// Calculate signed difference between two sequence numbers
    /// Handles wraparound correctly
    fn sequence_diff(&self, a: u32, b: u32) -> i64 {
        let diff = a.wrapping_sub(b) as i32;
        diff as i64
    }

    /// Get packet loss rate (0.0 - 1.0)
    pub fn loss_rate(&self) -> f32 {
        let total = self.packets_received + self.packets_lost;
        if total == 0 {
            0.0
        } else {
            self.packets_lost as f32 / total as f32
        }
    }

    /// Get total packets received
    pub fn packets_received(&self) -> u64 {
        self.packets_received
    }

    /// Get total packets lost
    pub fn packets_lost(&self) -> u64 {
        self.packets_lost
    }

    /// Get the highest sequence number seen
    pub fn highest_sequence(&self) -> u32 {
        self.highest_sequence
    }

    /// Check if a specific sequence number was received
    /// Only works for sequences within the window
    pub fn was_received(&self, sequence: u32) -> bool {
        let diff = self.sequence_diff(self.highest_sequence, sequence);
        if diff < 0 || diff >= 64 {
            false
        } else {
            let mask = 1u64 << diff;
            self.received_bitmap & mask != 0
        }
    }

    /// Reset statistics
    pub fn reset(&mut self) {
        self.last_sequence = None;
        self.highest_sequence = 0;
        self.received_bitmap = 0;
        self.packets_received = 0;
        self.packets_lost = 0;
    }
}

impl Default for SequenceTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequential_packets() {
        let mut tracker = SequenceTracker::new();

        // Sequential packets - no loss
        for i in 0..10 {
            let lost = tracker.record(i);
            assert!(lost.is_empty(), "Unexpected loss at sequence {}", i);
        }

        assert_eq!(tracker.packets_received(), 10);
        assert_eq!(tracker.packets_lost(), 0);
        assert_eq!(tracker.loss_rate(), 0.0);
    }

    #[test]
    fn test_single_packet_loss() {
        let mut tracker = SequenceTracker::new();

        tracker.record(0);
        tracker.record(1);
        // Skip 2
        let lost = tracker.record(3);

        assert_eq!(lost, vec![2]);
        assert_eq!(tracker.packets_lost(), 1);
    }

    #[test]
    fn test_multiple_packet_loss() {
        let mut tracker = SequenceTracker::new();

        tracker.record(0);
        // Skip 1, 2, 3
        let lost = tracker.record(4);

        assert_eq!(lost, vec![1, 2, 3]);
        assert_eq!(tracker.packets_lost(), 3);
    }

    #[test]
    fn test_out_of_order() {
        let mut tracker = SequenceTracker::new();

        tracker.record(0);
        tracker.record(1);
        let lost = tracker.record(3); // 2 appears lost
        assert_eq!(lost, vec![2]);
        assert_eq!(tracker.packets_lost(), 1);

        // Late arrival of packet 2
        let lost = tracker.record(2);
        assert!(lost.is_empty());
        assert_eq!(tracker.packets_lost(), 0); // Corrected
    }

    #[test]
    fn test_sequence_wraparound() {
        let mut tracker = SequenceTracker::new();

        tracker.record(u32::MAX - 2);
        tracker.record(u32::MAX - 1);
        tracker.record(u32::MAX);
        let lost = tracker.record(0); // Wraparound

        assert!(lost.is_empty());
        assert_eq!(tracker.packets_lost(), 0);
    }

    #[test]
    fn test_loss_rate_calculation() {
        let mut tracker = SequenceTracker::new();

        // 8 received, 2 lost (skip 3 and 7)
        for i in 0..10 {
            if i != 3 && i != 7 {
                tracker.record(i);
            }
        }

        // Actually trigger loss detection
        tracker.record(10);

        let rate = tracker.loss_rate();
        assert!(rate > 0.0 && rate < 0.5);
    }

    #[test]
    fn test_was_received() {
        let mut tracker = SequenceTracker::new();

        tracker.record(0);
        tracker.record(1);
        tracker.record(3); // 2 is lost

        assert!(tracker.was_received(0));
        assert!(tracker.was_received(1));
        assert!(!tracker.was_received(2));
        assert!(tracker.was_received(3));
    }

    #[test]
    fn test_reset() {
        let mut tracker = SequenceTracker::new();

        tracker.record(0);
        tracker.record(5); // 1-4 lost

        assert!(tracker.packets_lost() > 0);

        tracker.reset();

        assert_eq!(tracker.packets_received(), 0);
        assert_eq!(tracker.packets_lost(), 0);
        assert_eq!(tracker.loss_rate(), 0.0);
    }

    #[test]
    fn test_duplicate_packets() {
        let mut tracker = SequenceTracker::new();

        tracker.record(0);
        tracker.record(1);
        tracker.record(1); // Duplicate
        tracker.record(1); // Duplicate

        assert_eq!(tracker.packets_received(), 4);
        assert_eq!(tracker.packets_lost(), 0);
    }
}
