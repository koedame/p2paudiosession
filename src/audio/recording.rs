//! Audio recording functionality
//!
//! Records audio streams to WAV files.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use tracing::{info, warn};

use super::error::AudioError;

/// WAV file header constants
const RIFF_HEADER: &[u8] = b"RIFF";
const WAVE_HEADER: &[u8] = b"WAVE";
const FMT_HEADER: &[u8] = b"fmt ";
const DATA_HEADER: &[u8] = b"data";

/// Audio recorder for saving sessions to WAV files
pub struct Recorder {
    writer: Option<BufWriter<File>>,
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
    samples_written: Arc<AtomicU64>,
    recording: Arc<AtomicBool>,
    file_path: Option<String>,
}

impl Recorder {
    /// Create a new recorder
    pub fn new(sample_rate: u32, channels: u16, bits_per_sample: u16) -> Self {
        Self {
            writer: None,
            sample_rate,
            channels,
            bits_per_sample,
            samples_written: Arc::new(AtomicU64::new(0)),
            recording: Arc::new(AtomicBool::new(false)),
            file_path: None,
        }
    }

    /// Start recording to a file
    pub fn start<P: AsRef<Path>>(&mut self, path: P) -> Result<(), AudioError> {
        if self.recording.load(Ordering::SeqCst) {
            return Err(AudioError::RecordingError("Already recording".to_string()));
        }

        let path_str = path.as_ref().to_string_lossy().to_string();
        let file = File::create(&path)
            .map_err(|e| AudioError::RecordingError(format!("Failed to create file: {}", e)))?;

        let mut writer = BufWriter::new(file);

        // Write placeholder WAV header (will be updated when recording stops)
        write_wav_header(
            &mut writer,
            self.sample_rate,
            self.channels,
            self.bits_per_sample,
            0,
        )
        .map_err(|e| AudioError::RecordingError(format!("Failed to write header: {}", e)))?;

        self.writer = Some(writer);
        self.file_path = Some(path_str.clone());
        self.samples_written.store(0, Ordering::SeqCst);
        self.recording.store(true, Ordering::SeqCst);

        info!("Recording started: {}", path_str);
        Ok(())
    }

    /// Stop recording and finalize the file
    pub fn stop(&mut self) -> Result<RecordingInfo, AudioError> {
        if !self.recording.load(Ordering::SeqCst) {
            return Err(AudioError::RecordingError("Not recording".to_string()));
        }

        self.recording.store(false, Ordering::SeqCst);

        if let Some(mut writer) = self.writer.take() {
            let samples = self.samples_written.load(Ordering::SeqCst);
            let data_size = samples * (self.bits_per_sample as u64 / 8);

            // Seek back and update the header with correct sizes
            writer
                .flush()
                .map_err(|e| AudioError::RecordingError(format!("Failed to flush: {}", e)))?;

            // Get the underlying file and update header
            let _file = writer
                .into_inner()
                .map_err(|e| AudioError::RecordingError(format!("Failed to get file: {}", e)))?;

            // Rewrite header with correct size
            update_wav_header(
                self.file_path.as_ref().unwrap(),
                self.sample_rate,
                self.channels,
                self.bits_per_sample,
                data_size as u32,
            )
            .map_err(|e| AudioError::RecordingError(format!("Failed to update header: {}", e)))?;

            let duration_secs = samples as f64 / (self.sample_rate as f64 * self.channels as f64);

            info!(
                "Recording stopped: {} ({:.2}s)",
                self.file_path.as_ref().unwrap(),
                duration_secs
            );

            Ok(RecordingInfo {
                path: self.file_path.take().unwrap(),
                samples,
                duration_secs,
                file_size: 44 + data_size, // Header + data
            })
        } else {
            Err(AudioError::RecordingError(
                "No writer available".to_string(),
            ))
        }
    }

    /// Write audio samples (f32 format)
    pub fn write_samples(&mut self, samples: &[f32]) -> Result<(), AudioError> {
        if !self.recording.load(Ordering::SeqCst) {
            return Ok(()); // Silently ignore if not recording
        }

        if let Some(ref mut writer) = self.writer {
            // Convert f32 to the target bit depth
            match self.bits_per_sample {
                16 => {
                    for &sample in samples {
                        let value = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
                        writer.write_all(&value.to_le_bytes()).map_err(|e| {
                            AudioError::RecordingError(format!("Write failed: {}", e))
                        })?;
                    }
                }
                24 => {
                    for &sample in samples {
                        let value = (sample.clamp(-1.0, 1.0) * 8388607.0) as i32;
                        let bytes = value.to_le_bytes();
                        writer.write_all(&bytes[0..3]).map_err(|e| {
                            AudioError::RecordingError(format!("Write failed: {}", e))
                        })?;
                    }
                }
                32 => {
                    for &sample in samples {
                        writer.write_all(&sample.to_le_bytes()).map_err(|e| {
                            AudioError::RecordingError(format!("Write failed: {}", e))
                        })?;
                    }
                }
                _ => {
                    return Err(AudioError::RecordingError(format!(
                        "Unsupported bit depth: {}",
                        self.bits_per_sample
                    )));
                }
            }

            self.samples_written
                .fetch_add(samples.len() as u64, Ordering::SeqCst);
        }

        Ok(())
    }

    /// Check if recording is active
    pub fn is_recording(&self) -> bool {
        self.recording.load(Ordering::SeqCst)
    }

    /// Get number of samples written
    pub fn samples_written(&self) -> u64 {
        self.samples_written.load(Ordering::SeqCst)
    }

    /// Get recording duration in seconds
    pub fn duration_secs(&self) -> f64 {
        let samples = self.samples_written.load(Ordering::SeqCst);
        samples as f64 / (self.sample_rate as f64 * self.channels as f64)
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        if self.recording.load(Ordering::SeqCst) {
            if let Err(e) = self.stop() {
                warn!("Failed to stop recording on drop: {}", e);
            }
        }
    }
}

/// Information about a completed recording
#[derive(Debug, Clone)]
pub struct RecordingInfo {
    pub path: String,
    pub samples: u64,
    pub duration_secs: f64,
    pub file_size: u64,
}

/// Write WAV file header
fn write_wav_header<W: Write>(
    writer: &mut W,
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
    data_size: u32,
) -> std::io::Result<()> {
    let byte_rate = sample_rate * channels as u32 * bits_per_sample as u32 / 8;
    let block_align = channels * bits_per_sample / 8;
    let file_size = 36 + data_size;

    // RIFF header
    writer.write_all(RIFF_HEADER)?;
    writer.write_all(&file_size.to_le_bytes())?;
    writer.write_all(WAVE_HEADER)?;

    // fmt subchunk
    writer.write_all(FMT_HEADER)?;
    writer.write_all(&16u32.to_le_bytes())?; // Subchunk1 size
    let format = if bits_per_sample == 32 { 3u16 } else { 1u16 }; // 3 = IEEE float, 1 = PCM
    writer.write_all(&format.to_le_bytes())?;
    writer.write_all(&channels.to_le_bytes())?;
    writer.write_all(&sample_rate.to_le_bytes())?;
    writer.write_all(&byte_rate.to_le_bytes())?;
    writer.write_all(&block_align.to_le_bytes())?;
    writer.write_all(&bits_per_sample.to_le_bytes())?;

    // data subchunk
    writer.write_all(DATA_HEADER)?;
    writer.write_all(&data_size.to_le_bytes())?;

    Ok(())
}

/// Update WAV header with correct size (rewrites the file header)
fn update_wav_header(
    path: &str,
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
    data_size: u32,
) -> std::io::Result<()> {
    use std::io::{Seek, SeekFrom};

    let mut file = std::fs::OpenOptions::new().write(true).open(path)?;

    file.seek(SeekFrom::Start(0))?;

    let mut writer = BufWriter::new(&mut file);
    write_wav_header(
        &mut writer,
        sample_rate,
        channels,
        bits_per_sample,
        data_size,
    )?;
    writer.flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_recorder_creation() {
        let recorder = Recorder::new(48000, 2, 16);
        assert!(!recorder.is_recording());
        assert_eq!(recorder.samples_written(), 0);
    }

    #[test]
    fn test_record_and_stop() {
        let mut recorder = Recorder::new(48000, 1, 16);
        let path = "/tmp/test_recording.wav";

        // Start recording
        recorder.start(path).unwrap();
        assert!(recorder.is_recording());

        // Write some samples
        let samples: Vec<f32> = (0..4800).map(|i| (i as f32 * 0.01).sin()).collect();
        recorder.write_samples(&samples).unwrap();

        // Stop recording
        let info = recorder.stop().unwrap();
        assert!(!recorder.is_recording());
        assert_eq!(info.samples, 4800);
        assert!(info.duration_secs > 0.0);

        // Verify file exists
        assert!(Path::new(path).exists());

        // Clean up
        fs::remove_file(path).ok();
    }

    #[test]
    fn test_wav_header() {
        let mut buffer = Vec::new();
        write_wav_header(&mut buffer, 48000, 2, 16, 1000).unwrap();

        assert_eq!(&buffer[0..4], b"RIFF");
        assert_eq!(&buffer[8..12], b"WAVE");
        assert_eq!(&buffer[12..16], b"fmt ");
    }
}
