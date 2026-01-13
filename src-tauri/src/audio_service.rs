//! Audio service - manages audio engine in a dedicated thread
//!
//! cpal::Stream is not Send, so AudioEngine must run in a dedicated thread.
//! This service provides a thread-safe interface via channels.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crossbeam_channel::{bounded, Receiver, Sender};
use ringbuf::traits::Producer;
use tracing::{error, info, warn};

use jamjam::audio::{AudioConfig, AudioEngine, AudioEvent, DeviceId};

/// Commands sent to the audio thread
#[derive(Debug)]
pub enum AudioCommand {
    /// Start audio capture and playback
    Start {
        input_device: Option<String>,
        output_device: Option<String>,
        config: AudioConfig,
    },
    /// Stop audio
    Stop,
    /// Enable/disable local monitoring (hear yourself)
    SetLocalMonitoring(bool),
    /// Enqueue remote audio for playback (from network)
    EnqueueRemoteAudio(Vec<f32>),
    /// Switch input device while running
    SetInputDevice(Option<String>),
    /// Switch output device while running
    SetOutputDevice(Option<String>),
    /// Shutdown the audio thread
    Shutdown,
}

/// Events sent from the audio thread to notify about device changes
#[derive(Debug, Clone)]
pub enum DeviceEvent {
    /// Input device was disconnected and fallback occurred
    InputDeviceDisconnected {
        fallback_device: Option<String>,
    },
    /// Output device was disconnected and fallback occurred
    OutputDeviceDisconnected {
        fallback_device: Option<String>,
    },
}

/// Response from audio thread
#[derive(Debug)]
pub enum AudioResponse {
    Ok,
    Error(String),
}

/// Handle to the audio service
pub struct AudioServiceHandle {
    cmd_tx: Sender<AudioCommand>,
    resp_rx: Receiver<AudioResponse>,
    thread_handle: Option<JoinHandle<()>>,
    running: Arc<AtomicBool>,
    local_monitoring: Arc<AtomicBool>,
    /// Receiver for device events (to be polled by main thread)
    device_event_rx: mpsc::Receiver<DeviceEvent>,
}

impl AudioServiceHandle {
    /// Create and start a new audio service
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = bounded::<AudioCommand>(16);
        let (resp_tx, resp_rx) = bounded::<AudioResponse>(16);
        let (device_event_tx, device_event_rx) = mpsc::channel::<DeviceEvent>();
        let running = Arc::new(AtomicBool::new(false));
        let local_monitoring = Arc::new(AtomicBool::new(false));
        let running_clone = running.clone();
        let local_monitoring_clone = local_monitoring.clone();

        let thread_handle = thread::spawn(move || {
            audio_thread_main(
                cmd_rx,
                resp_tx,
                device_event_tx,
                running_clone,
                local_monitoring_clone,
            );
        });

        Self {
            cmd_tx,
            resp_rx,
            thread_handle: Some(thread_handle),
            running,
            local_monitoring,
            device_event_rx,
        }
    }

    /// Start audio with the given devices and config
    pub fn start(
        &self,
        input_device: Option<String>,
        output_device: Option<String>,
        config: AudioConfig,
    ) -> Result<(), String> {
        self.cmd_tx
            .send(AudioCommand::Start {
                input_device,
                output_device,
                config,
            })
            .map_err(|e| format!("Failed to send start command: {}", e))?;

        match self.resp_rx.recv() {
            Ok(AudioResponse::Ok) => Ok(()),
            Ok(AudioResponse::Error(e)) => Err(e),
            Err(e) => Err(format!("Failed to receive response: {}", e)),
        }
    }

    /// Stop audio
    pub fn stop(&self) -> Result<(), String> {
        self.cmd_tx
            .send(AudioCommand::Stop)
            .map_err(|e| format!("Failed to send stop command: {}", e))?;

        match self.resp_rx.recv() {
            Ok(AudioResponse::Ok) => Ok(()),
            Ok(AudioResponse::Error(e)) => Err(e),
            Err(e) => Err(format!("Failed to receive response: {}", e)),
        }
    }

    /// Set local monitoring (hear yourself)
    pub fn set_local_monitoring(&self, enabled: bool) -> Result<(), String> {
        self.cmd_tx
            .send(AudioCommand::SetLocalMonitoring(enabled))
            .map_err(|e| format!("Failed to send monitoring command: {}", e))?;

        match self.resp_rx.recv() {
            Ok(AudioResponse::Ok) => Ok(()),
            Ok(AudioResponse::Error(e)) => Err(e),
            Err(e) => Err(format!("Failed to receive response: {}", e)),
        }
    }

    /// Check if audio is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Check if local monitoring is enabled
    pub fn is_local_monitoring(&self) -> bool {
        self.local_monitoring.load(Ordering::SeqCst)
    }

    /// Enqueue remote audio for playback (non-blocking, drops on overflow)
    ///
    /// This is called from the Session's async receive loop to feed
    /// received audio to the AudioEngine for playback.
    pub fn enqueue_remote_audio(&self, samples: Vec<f32>) {
        // Use try_send to avoid blocking in async context
        // If channel is full, drop the audio (better than blocking)
        let _ = self.cmd_tx.try_send(AudioCommand::EnqueueRemoteAudio(samples));
    }

    /// Switch input device while audio is running
    pub fn set_input_device(&self, device_id: Option<String>) -> Result<(), String> {
        self.cmd_tx
            .send(AudioCommand::SetInputDevice(device_id))
            .map_err(|e| format!("Failed to send set_input_device command: {}", e))?;

        match self.resp_rx.recv() {
            Ok(AudioResponse::Ok) => Ok(()),
            Ok(AudioResponse::Error(e)) => Err(e),
            Err(e) => Err(format!("Failed to receive response: {}", e)),
        }
    }

    /// Switch output device while audio is running
    pub fn set_output_device(&self, device_id: Option<String>) -> Result<(), String> {
        self.cmd_tx
            .send(AudioCommand::SetOutputDevice(device_id))
            .map_err(|e| format!("Failed to send set_output_device command: {}", e))?;

        match self.resp_rx.recv() {
            Ok(AudioResponse::Ok) => Ok(()),
            Ok(AudioResponse::Error(e)) => Err(e),
            Err(e) => Err(format!("Failed to receive response: {}", e)),
        }
    }

    /// Try to receive a device event (non-blocking)
    pub fn try_recv_device_event(&self) -> Option<DeviceEvent> {
        self.device_event_rx.try_recv().ok()
    }
}

impl Default for AudioServiceHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AudioServiceHandle {
    fn drop(&mut self) {
        // Send shutdown command
        let _ = self.cmd_tx.send(AudioCommand::Shutdown);

        // Wait for thread to finish
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

/// Create a capture callback that handles local monitoring
fn create_capture_callback(
    playback_producer: Option<jamjam::audio::SharedPlaybackProducer>,
    local_monitoring: Arc<AtomicBool>,
) -> impl Fn(&[f32], u64) + Send + Sync + 'static {
    move |samples: &[f32], _timestamp: u64| {
        // If local monitoring is enabled, write to playback buffer
        if local_monitoring.load(Ordering::SeqCst) {
            if let Some(ref producer) = playback_producer {
                if let Ok(mut prod) = producer.try_lock() {
                    for &sample in samples {
                        // Ignore overflow - better to drop samples than block
                        let _ = prod.try_push(sample);
                    }
                }
            }
        }
    }
}

/// Main function for the audio thread
fn audio_thread_main(
    cmd_rx: Receiver<AudioCommand>,
    resp_tx: Sender<AudioResponse>,
    device_event_tx: mpsc::Sender<DeviceEvent>,
    running: Arc<AtomicBool>,
    local_monitoring: Arc<AtomicBool>,
) {
    info!("Audio thread started");

    let mut engine: Option<AudioEngine> = None;
    let mut audio_event_rx: Option<mpsc::Receiver<AudioEvent>> = None;

    loop {
        // Non-blocking check for audio events (device disconnection)
        if let Some(ref rx) = audio_event_rx {
            while let Ok(event) = rx.try_recv() {
                match event {
                    AudioEvent::InputDeviceDisconnected => {
                        warn!("Input device disconnected, attempting fallback to default");
                        if let Some(ref mut eng) = engine {
                            let playback_producer = eng.playback_producer();
                            let callback =
                                create_capture_callback(playback_producer, local_monitoring.clone());

                            match eng.set_input_device(None, callback) {
                                Ok(_) => {
                                    info!("Fallback to default input device successful");
                                    let _ = device_event_tx.send(DeviceEvent::InputDeviceDisconnected {
                                        fallback_device: Some("default".to_string()),
                                    });
                                }
                                Err(e) => {
                                    error!("Fallback to default input device failed: {}", e);
                                    let _ = device_event_tx.send(DeviceEvent::InputDeviceDisconnected {
                                        fallback_device: None,
                                    });
                                }
                            }
                        }
                    }
                    AudioEvent::OutputDeviceDisconnected => {
                        warn!("Output device disconnected, attempting fallback to default");
                        if let Some(ref mut eng) = engine {
                            match eng.set_output_device(None) {
                                Ok(_) => {
                                    info!("Fallback to default output device successful");
                                    let _ = device_event_tx.send(DeviceEvent::OutputDeviceDisconnected {
                                        fallback_device: Some("default".to_string()),
                                    });
                                }
                                Err(e) => {
                                    error!("Fallback to default output device failed: {}", e);
                                    let _ = device_event_tx.send(DeviceEvent::OutputDeviceDisconnected {
                                        fallback_device: None,
                                    });
                                }
                            }
                        }
                    }
                    AudioEvent::StreamError(err) => {
                        error!("Stream error: {}", err);
                    }
                }
            }
        }

        // Use recv_timeout to allow periodic checking of audio events
        match cmd_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(AudioCommand::Start {
                input_device,
                output_device,
                config,
            }) => {
                info!(
                    "Starting audio: input={:?}, output={:?}, config={:?}",
                    input_device, output_device, config
                );

                // Stop existing engine if any
                engine = None;
                audio_event_rx = None;

                let mut new_engine = AudioEngine::new(config);

                // Set up event channel for device disconnection notifications
                let (event_tx, event_rx) = mpsc::channel::<AudioEvent>();
                new_engine.set_event_sender(event_tx);
                audio_event_rx = Some(event_rx);

                // Start playback first (creates the ring buffer)
                let output_device_id = output_device.map(DeviceId);
                if let Err(e) = new_engine.start_playback(output_device_id.as_ref()) {
                    error!("Failed to start playback: {}", e);
                    let _ = resp_tx.send(AudioResponse::Error(e.to_string()));
                    continue;
                }

                // Get shared playback producer and local monitoring flag
                let playback_producer = new_engine.playback_producer();
                let capture_callback =
                    create_capture_callback(playback_producer, local_monitoring.clone());

                // Start capture with callback that handles local monitoring
                let input_device_id = input_device.map(DeviceId);

                if let Err(e) = new_engine.start_capture(input_device_id.as_ref(), capture_callback)
                {
                    error!("Failed to start capture: {}", e);
                    new_engine.stop_playback();
                    let _ = resp_tx.send(AudioResponse::Error(e.to_string()));
                    continue;
                }

                engine = Some(new_engine);
                running.store(true, Ordering::SeqCst);
                info!("Audio started successfully");
                let _ = resp_tx.send(AudioResponse::Ok);
            }

            Ok(AudioCommand::Stop) => {
                info!("Stopping audio");
                if let Some(ref mut eng) = engine {
                    eng.stop_capture();
                    eng.stop_playback();
                }
                engine = None;
                audio_event_rx = None;
                running.store(false, Ordering::SeqCst);
                info!("Audio stopped");
                let _ = resp_tx.send(AudioResponse::Ok);
            }

            Ok(AudioCommand::SetLocalMonitoring(enabled)) => {
                info!("Setting local monitoring: {}", enabled);
                local_monitoring.store(enabled, Ordering::SeqCst);
                let _ = resp_tx.send(AudioResponse::Ok);
            }

            Ok(AudioCommand::EnqueueRemoteAudio(samples)) => {
                // Feed remote audio to playback buffer
                if let Some(ref engine) = engine {
                    engine.enqueue_playback(&samples);
                }
                // No response needed - this is fire-and-forget
            }

            Ok(AudioCommand::SetInputDevice(device_id)) => {
                info!("Switching input device to: {:?}", device_id);
                if let Some(ref mut eng) = engine {
                    let playback_producer = eng.playback_producer();
                    let callback =
                        create_capture_callback(playback_producer, local_monitoring.clone());
                    let device_id_ref = device_id.map(DeviceId);

                    match eng.set_input_device(device_id_ref.as_ref(), callback) {
                        Ok(_) => {
                            info!("Input device switched successfully");
                            let _ = resp_tx.send(AudioResponse::Ok);
                        }
                        Err(e) => {
                            error!("Failed to switch input device: {}", e);
                            let _ = resp_tx.send(AudioResponse::Error(e.to_string()));
                        }
                    }
                } else {
                    let _ = resp_tx.send(AudioResponse::Error("Audio not running".to_string()));
                }
            }

            Ok(AudioCommand::SetOutputDevice(device_id)) => {
                info!("Switching output device to: {:?}", device_id);
                if let Some(ref mut eng) = engine {
                    let device_id_ref = device_id.map(DeviceId);

                    match eng.set_output_device(device_id_ref.as_ref()) {
                        Ok(_) => {
                            info!("Output device switched successfully");
                            let _ = resp_tx.send(AudioResponse::Ok);
                        }
                        Err(e) => {
                            error!("Failed to switch output device: {}", e);
                            let _ = resp_tx.send(AudioResponse::Error(e.to_string()));
                        }
                    }
                } else {
                    let _ = resp_tx.send(AudioResponse::Error("Audio not running".to_string()));
                }
            }

            Ok(AudioCommand::Shutdown) => {
                info!("Audio thread shutting down");
                if let Some(ref mut eng) = engine {
                    eng.stop_capture();
                    eng.stop_playback();
                }
                running.store(false, Ordering::SeqCst);
                break;
            }

            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                // Timeout - continue to check audio events
                continue;
            }

            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                // Channel closed, exit
                warn!("Audio command channel closed");
                break;
            }
        }
    }

    info!("Audio thread exited");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_service_creation() {
        let service = AudioServiceHandle::new();
        assert!(!service.is_running());
        assert!(!service.is_local_monitoring());
    }

    #[test]
    fn test_set_local_monitoring() {
        let service = AudioServiceHandle::new();
        assert!(service.set_local_monitoring(true).is_ok());
        assert!(service.is_local_monitoring());
        assert!(service.set_local_monitoring(false).is_ok());
        assert!(!service.is_local_monitoring());
    }

    #[test]
    fn test_set_input_device_not_running() {
        let service = AudioServiceHandle::new();
        let result = service.set_input_device(Some("test-device".to_string()));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Audio not running"));
    }

    #[test]
    fn test_set_output_device_not_running() {
        let service = AudioServiceHandle::new();
        let result = service.set_output_device(Some("test-device".to_string()));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Audio not running"));
    }

    #[test]
    fn test_set_input_device_with_none_not_running() {
        let service = AudioServiceHandle::new();
        let result = service.set_input_device(None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Audio not running"));
    }

    #[test]
    fn test_set_output_device_with_none_not_running() {
        let service = AudioServiceHandle::new();
        let result = service.set_output_device(None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Audio not running"));
    }
}
