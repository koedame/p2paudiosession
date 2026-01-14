/**
 * SettingsScreen - Audio device settings
 */

import { useEffect, useState } from "react";
import {
  AudioDeviceInfo,
  audioListInputDevices,
  audioListOutputDevices,
  audioSetInputDevice,
  audioSetOutputDevice,
  audioGetCurrentDevices,
} from "../lib/tauri";
import { DeviceSelector } from "../components/DeviceSelector";
import "./SettingsScreen.css";

export interface SettingsScreenProps {
  onBack: () => void;
}

export function SettingsScreen({ onBack }: SettingsScreenProps) {
  const [inputDevices, setInputDevices] = useState<AudioDeviceInfo[]>([]);
  const [outputDevices, setOutputDevices] = useState<AudioDeviceInfo[]>([]);
  const [selectedInputId, setSelectedInputId] = useState<string | null>(null);
  const [selectedOutputId, setSelectedOutputId] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Load devices on mount
  useEffect(() => {
    const loadDevices = async () => {
      try {
        setIsLoading(true);
        setError(null);

        const [inputs, outputs, current] = await Promise.all([
          audioListInputDevices(),
          audioListOutputDevices(),
          audioGetCurrentDevices(),
        ]);

        setInputDevices(inputs);
        setOutputDevices(outputs);
        setSelectedInputId(current.input_device_id);
        setSelectedOutputId(current.output_device_id);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setIsLoading(false);
      }
    };

    loadDevices();
  }, []);

  const handleInputChange = async (deviceId: string) => {
    try {
      setError(null);
      await audioSetInputDevice(deviceId);
      setSelectedInputId(deviceId);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleOutputChange = async (deviceId: string) => {
    try {
      setError(null);
      await audioSetOutputDevice(deviceId);
      setSelectedOutputId(deviceId);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  return (
    <div className="settings-screen">
      <header className="settings-header">
        <button
          className="settings-header__back-btn"
          onClick={onBack}
          aria-label="Back"
        >
          &#8592;
        </button>
        <h1 className="settings-header__title">Settings</h1>
        <div className="settings-header__spacer" />
      </header>

      <main className="settings-content">
        <div className="settings-card">
          <h2 className="settings-card__title">Audio Devices</h2>

          {error && <div className="settings-error">{error}</div>}

          <div className="settings-card__devices">
            <DeviceSelector
              type="input"
              devices={inputDevices}
              selectedDeviceId={selectedInputId}
              onDeviceChange={handleInputChange}
              isLoading={isLoading}
            />

            <DeviceSelector
              type="output"
              devices={outputDevices}
              selectedDeviceId={selectedOutputId}
              onDeviceChange={handleOutputChange}
              isLoading={isLoading}
            />
          </div>
        </div>
      </main>
    </div>
  );
}

export default SettingsScreen;
