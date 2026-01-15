/**
 * SettingsScreen - Audio device settings
 */

import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
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
  configLoad,
  configSave,
  configListPresets,
  configSetPreset,
  type AppConfig,
  type PresetInfo,
  type AudioPresetId,
} from "../lib/tauri";
import { DeviceSelector } from "../components/DeviceSelector";
import { useTheme } from "../hooks/useTheme";
import "./SettingsScreen.css";

export interface SettingsScreenProps {
  onBack: () => void;
}

// Preset display information
const PRESET_DISPLAY: Record<
  AudioPresetId,
  { name: string; description: string; useCase: string }
> = {
  "zero-latency": {
    name: "Zero Latency",
    description: "Jitter buffer off, 32 samples",
    useCase: "Fiber connections (Japan domestic)",
  },
  "ultra-low-latency": {
    name: "Ultra Low Latency",
    description: "1 frame buffer, 64 samples",
    useCase: "LAN sessions",
  },
  balanced: {
    name: "Balanced",
    description: "4 frame buffer, 128 samples",
    useCase: "Typical internet",
  },
  "high-quality": {
    name: "High Quality",
    description: "8 frame buffer, 256 samples",
    useCase: "Recording / High-speed connections",
  },
};

export function SettingsScreen({ onBack }: SettingsScreenProps) {
  const { t } = useTranslation();
  const { theme, setTheme } = useTheme();
  const [inputDevices, setInputDevices] = useState<AudioDeviceInfo[]>([]);
  const [outputDevices, setOutputDevices] = useState<AudioDeviceInfo[]>([]);
  const [selectedInputId, setSelectedInputId] = useState<string | null>(null);
  const [selectedOutputId, setSelectedOutputId] = useState<string | null>(null);
  const [bufferSize, setBufferSize] = useState<number>(64);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [presets, setPresets] = useState<PresetInfo[]>([]);
  const [selectedPreset, setSelectedPreset] = useState<AudioPresetId>("balanced");

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

        const [inputs, outputs, current, currentBufferSize, presetList, config] =
          await Promise.all([
            audioListInputDevices(),
            audioListOutputDevices(),
            audioGetCurrentDevices(),
            audioGetBufferSize(),
            configListPresets(),
            configLoad().catch(() => null),
          ]);

        setBufferSize(currentBufferSize);
        setPresets(presetList);
        if (config?.preset) {
          setSelectedPreset(config.preset);
        }

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

  // Save current settings to config file
  const saveConfig = async (inputId: string | null, outputId: string | null, bufSize: number) => {
    try {
      const config = await configLoad().catch(() => ({
        input_device_id: null,
        output_device_id: null,
        buffer_size: 64,
        signaling_server_url: null,
      } as AppConfig));

      await configSave({
        ...config,
        input_device_id: inputId,
        output_device_id: outputId,
        buffer_size: bufSize,
      });
    } catch (e) {
      console.error("Failed to save config:", e);
    }
  };

  const handleInputChange = async (deviceId: string) => {
    try {
      setError(null);
      // Always save to AudioState
      await audioSetInputDevice(deviceId);
      setSelectedInputId(deviceId);

      // Save to config file
      await saveConfig(deviceId, selectedOutputId, bufferSize);

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

      // Save to config file
      await saveConfig(selectedInputId, deviceId, bufferSize);

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

      // Save to config file
      await saveConfig(selectedInputId, selectedOutputId, newSize);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handlePresetChange = async (presetId: AudioPresetId) => {
    try {
      setError(null);
      // Set preset and apply its settings (buffer size is auto-applied by backend)
      const appliedPreset = await configSetPreset(presetId);
      setSelectedPreset(presetId);
      setBufferSize(appliedPreset.buffer_size);

      // Also update the audio system's buffer size
      await audioSetBufferSize(appliedPreset.buffer_size);
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
        {/* Preset Selection */}
        <div className="settings-card">
          <h2 className="settings-card__title">Audio Preset</h2>
          <p className="settings-card__hint">
            Select a preset based on your network environment. This will
            automatically configure buffer settings.
          </p>

          <div className="preset-selector">
            {presets.map((preset) => {
              const display = PRESET_DISPLAY[preset.id];
              const isSelected = preset.id === selectedPreset;
              return (
                <button
                  key={preset.id}
                  className={`preset-option ${isSelected ? "preset-option--selected" : ""}`}
                  onClick={() => handlePresetChange(preset.id)}
                  disabled={isLoading}
                >
                  <div className="preset-option__header">
                    <span className="preset-option__name">{display.name}</span>
                    {isSelected && <span className="preset-option__check">✓</span>}
                  </div>
                  <div className="preset-option__details">
                    <span className="preset-option__desc">{display.description}</span>
                    <span className="preset-option__usecase">{display.useCase}</span>
                  </div>
                </button>
              );
            })}
          </div>
        </div>

        {/* Audio Devices */}
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

        {/* Theme Selection */}
        <div className="settings-card">
          <h2 className="settings-card__title">{t("settings.display.title")}</h2>
          <p className="settings-card__hint">{t("settings.display.theme")}</p>

          <div className="theme-selector">
            <button
              className={`theme-option ${theme === "dark" ? "theme-option--selected" : ""}`}
              onClick={() => setTheme("dark")}
            >
              <span className="theme-option__icon">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
                </svg>
              </span>
              <span className="theme-option__label">{t("settings.display.themeDark")}</span>
            </button>
            <button
              className={`theme-option ${theme === "light" ? "theme-option--selected" : ""}`}
              onClick={() => setTheme("light")}
            >
              <span className="theme-option__icon">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <circle cx="12" cy="12" r="5" />
                  <line x1="12" y1="1" x2="12" y2="3" />
                  <line x1="12" y1="21" x2="12" y2="23" />
                  <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
                  <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
                  <line x1="1" y1="12" x2="3" y2="12" />
                  <line x1="21" y1="12" x2="23" y2="12" />
                  <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
                  <line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
                </svg>
              </span>
              <span className="theme-option__label">{t("settings.display.themeLight")}</span>
            </button>
            <button
              className={`theme-option ${theme === "system" ? "theme-option--selected" : ""}`}
              onClick={() => setTheme("system")}
            >
              <span className="theme-option__icon">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
                  <line x1="8" y1="21" x2="16" y2="21" />
                  <line x1="12" y1="17" x2="12" y2="21" />
                </svg>
              </span>
              <span className="theme-option__label">{t("settings.display.themeSystem")}</span>
            </button>
          </div>
        </div>
      </main>
    </div>
  );
}

export default SettingsScreen;
