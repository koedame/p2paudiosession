//! Audio device enumeration and management

use cpal::traits::{DeviceTrait, HostTrait};

/// Unique identifier for an audio device
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeviceId(pub String);

/// Information about an audio device
#[derive(Debug, Clone)]
pub struct AudioDevice {
    pub id: DeviceId,
    pub name: String,
    pub is_default: bool,
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
                    Some(AudioDevice {
                        id: DeviceId(name.clone()),
                        name,
                        is_default,
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
                    Some(AudioDevice {
                        id: DeviceId(name.clone()),
                        name,
                        is_default,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
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
