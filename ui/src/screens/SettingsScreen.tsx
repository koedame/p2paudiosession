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
  audioGetBufferSize,
  audioSetBufferSize,
  streamingStatus,
  streamingSetInputDevice,
  streamingSetOutputDevice,
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
  const [bufferSize, setBufferSize] = useState<number>(64);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Buffer size options: 32, 64, 128, 256
  const bufferSizeOptions = [32, 64, 128, 256];
  const bufferSizeToIndex = (size: number) =>
    bufferSizeOptions.indexOf(size) !== -1
      ? bufferSizeOptions.indexOf(size)
      : 1;
  const indexToBufferSize = (index: number) =>
    bufferSizeOptions[index] ?? 64;

  // Load devices and settings on mount
  useEffect(() => {
    const loadDevices = async () => {
      try {
        setIsLoading(true);
        setError(null);

        const [inputs, outputs, current, currentBufferSize] = await Promise.all(
          [
            audioListInputDevices(),
            audioListOutputDevices(),
            audioGetCurrentDevices(),
            audioGetBufferSize(),
          ]
        );

        setBufferSize(currentBufferSize);

        setInputDevices(inputs);
        setOutputDevices(outputs);

        // If no device is selected, use the default device and save it
        let inputId = current.input_device_id;
        let outputId = current.output_device_id;

        if (!inputId && inputs.length > 0) {
          const defaultInput = inputs.find((d) => d.is_default) || inputs[0];
          inputId = defaultInput.id;
          await audioSetInputDevice(inputId);
        }

        if (!outputId && outputs.length > 0) {
          const defaultOutput = outputs.find((d) => d.is_default) || outputs[0];
          outputId = defaultOutput.id;
          await audioSetOutputDevice(outputId);
        }

        setSelectedInputId(inputId);
        setSelectedOutputId(outputId);
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
      // Always save to AudioState
      await audioSetInputDevice(deviceId);
      setSelectedInputId(deviceId);

      // If streaming is active, also update the running stream
      try {
        const status = await streamingStatus();
        if (status.is_active) {
          await streamingSetInputDevice(deviceId);
        }
      } catch {
        // Ignore errors from streaming status check
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleOutputChange = async (deviceId: string) => {
    try {
      setError(null);
      // Always save to AudioState
      await audioSetOutputDevice(deviceId);
      setSelectedOutputId(deviceId);

      // If streaming is active, also update the running stream
      try {
        const status = await streamingStatus();
        if (status.is_active) {
          await streamingSetOutputDevice(deviceId);
        }
      } catch {
        // Ignore errors from streaming status check
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleBufferSizeChange = async (newSize: number) => {
    try {
      setError(null);
      await audioSetBufferSize(newSize);
      setBufferSize(newSize);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  // Calculate latency in ms for display
  const calculateLatencyMs = (samples: number) =>
    ((samples / 48000) * 1000).toFixed(2);

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

        <div className="settings-card">
          <h2 className="settings-card__title">Audio Buffer</h2>
          <p className="settings-card__hint">
            Lower values reduce latency but may cause crackling. Higher values
            are more stable but add latency.
          </p>

          <div className="buffer-slider">
            <div className="buffer-slider__labels">
              {bufferSizeOptions.map((size) => (
                <span
                  key={size}
                  className={`buffer-slider__label ${
                    size === bufferSize ? "buffer-slider__label--active" : ""
                  }`}
                >
                  {size}
                </span>
              ))}
            </div>
            <input
              type="range"
              min="0"
              max="3"
              step="1"
              value={bufferSizeToIndex(bufferSize)}
              onChange={(e) =>
                handleBufferSizeChange(indexToBufferSize(Number(e.target.value)))
              }
              className="buffer-slider__input"
              disabled={isLoading}
            />
            <div className="buffer-slider__info">
              <span className="buffer-slider__value">
                {bufferSize} samples ({calculateLatencyMs(bufferSize)} ms)
              </span>
            </div>
          </div>
        </div>
      </main>
    </div>
  );
}

export default SettingsScreen;
