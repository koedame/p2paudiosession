/**
 * DeviceSelector - Audio device dropdown selector
 */

import { AudioDeviceInfo } from "../../lib/tauri";
import "./DeviceSelector.css";

export interface DeviceSelectorProps {
  /** Type of device (input or output) */
  type: "input" | "output";
  /** List of available devices */
  devices: AudioDeviceInfo[];
  /** Currently selected device ID */
  selectedDeviceId: string | null;
  /** Callback when device is changed */
  onDeviceChange: (deviceId: string) => void;
  /** Whether the selector is disabled */
  disabled?: boolean;
  /** Whether devices are loading */
  isLoading?: boolean;
}

export function DeviceSelector({
  type,
  devices,
  selectedDeviceId,
  onDeviceChange,
  disabled = false,
  isLoading = false,
}: DeviceSelectorProps) {
  const label = type === "input" ? "Input" : "Output";

  const handleChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    onDeviceChange(e.target.value);
  };

  // Find the selected device, or use default if not set
  const currentValue =
    selectedDeviceId ||
    devices.find((d) => d.is_default)?.id ||
    devices[0]?.id ||
    "";

  return (
    <div className="device-selector">
      <label className="device-selector__label">{label}</label>
      <div className="device-selector__wrapper">
        <select
          className="device-selector__select"
          value={currentValue}
          onChange={handleChange}
          disabled={disabled || isLoading || devices.length === 0}
        >
          {isLoading ? (
            <option value="">Loading...</option>
          ) : devices.length === 0 ? (
            <option value="">No devices found</option>
          ) : (
            devices.map((device) => (
              <option key={device.id} value={device.id}>
                {device.name}
                {device.is_default ? " (Default)" : ""}
              </option>
            ))
          )}
        </select>
        <span className="device-selector__arrow">&#9662;</span>
      </div>
    </div>
  );
}

export default DeviceSelector;
