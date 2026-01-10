//! Audio device enumeration and management

use cpal::traits::{DeviceTrait, HostTrait};

/// Unique identifier for an audio device
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeviceId(pub String);

/// Information about an audio device
#[derive(Debug, Clone)]
pub struct AudioDevice {
    /// Device identifier
    pub id: DeviceId,
    /// Display name
    pub name: String,
    /// Supported sample rates (Hz)
    pub supported_sample_rates: Vec<u32>,
    /// Supported channel counts
    pub supported_channels: Vec<u16>,
    /// Whether this is the default device
    pub is_default: bool,
    /// ASIO support (Windows only)
    pub is_asio: bool,
}

/// List available input (capture) devices
pub fn list_input_devices() -> Vec<AudioDevice> {
    let host = cpal::default_host();
    let default_device = host.default_input_device();
    let default_name = default_device.as_ref().and_then(|d| d.name().ok());

    host.input_devices()
        .map(|devices| {
            devices
                .filter_map(|device| {
                    let name = device.name().ok()?;
                    let is_default = default_name.as_ref() == Some(&name);
                    let (sample_rates, channels) = get_device_capabilities(&device);
                    Some(AudioDevice {
                        id: DeviceId(name.clone()),
                        name,
                        supported_sample_rates: sample_rates,
                        supported_channels: channels,
                        is_default,
                        is_asio: is_asio_device(&device),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// List available output (playback) devices
pub fn list_output_devices() -> Vec<AudioDevice> {
    let host = cpal::default_host();
    let default_device = host.default_output_device();
    let default_name = default_device.as_ref().and_then(|d| d.name().ok());

    host.output_devices()
        .map(|devices| {
            devices
                .filter_map(|device| {
                    let name = device.name().ok()?;
                    let is_default = default_name.as_ref() == Some(&name);
                    let (sample_rates, channels) = get_device_capabilities(&device);
                    Some(AudioDevice {
                        id: DeviceId(name.clone()),
                        name,
                        supported_sample_rates: sample_rates,
                        supported_channels: channels,
                        is_default,
                        is_asio: is_asio_device(&device),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Get supported sample rates and channel counts for a device
fn get_device_capabilities(device: &cpal::Device) -> (Vec<u32>, Vec<u16>) {
    let mut sample_rates = Vec::new();
    let mut channels = Vec::new();

    // Try input configs first, then output configs
    let configs: Vec<_> = device.supported_input_configs()
        .into_iter()
        .flatten()
        .chain(device.supported_output_configs().into_iter().flatten())
        .collect();

    for config in configs {
        // Add common sample rates that fall within the supported range
        for rate in &[44100u32, 48000, 96000, 192000] {
            if *rate >= config.min_sample_rate().0 && *rate <= config.max_sample_rate().0 {
                if !sample_rates.contains(rate) {
                    sample_rates.push(*rate);
                }
            }
        }
        let ch = config.channels();
        if !channels.contains(&ch) {
            channels.push(ch);
        }
    }

    sample_rates.sort();
    channels.sort();

    // Provide defaults if nothing was detected
    if sample_rates.is_empty() {
        sample_rates = vec![44100, 48000];
    }
    if channels.is_empty() {
        channels = vec![1, 2];
    }

    (sample_rates, channels)
}

/// Check if device is an ASIO device (Windows only)
#[cfg(target_os = "windows")]
fn is_asio_device(device: &cpal::Device) -> bool {
    device.name().map(|n| n.contains("ASIO")).unwrap_or(false)
}

#[cfg(not(target_os = "windows"))]
fn is_asio_device(_device: &cpal::Device) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_devices_does_not_panic() {
        // This test ensures device enumeration doesn't panic
        // Actual device availability depends on the system
        let _inputs = list_input_devices();
        let _outputs = list_output_devices();
    }
}
