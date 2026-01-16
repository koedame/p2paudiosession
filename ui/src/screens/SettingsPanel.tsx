/**
 * SettingsPanel - Audio device and display settings content
 *
 * This is the content for the settings side panel.
 * For use inside the SidePanel component.
 */

import { useEffect, useState, useCallback } from "react";
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
  streamingStart,
  streamingStop,
  streamingSetInputDevice,
  streamingSetOutputDevice,
  configLoad,
  configSave,
  configGetPeerName,
  configSetPeerName,
  type AppConfig,
} from "../lib/tauri";
import { DeviceSelector } from "../components/DeviceSelector";
import { useTheme } from "../hooks/useTheme";
import "./SettingsPanel.css";

export interface SettingsPanelProps {
  /** Callback when settings are changed (to trigger reload in parent) */
  onSettingsChange?: () => void;
}

export function SettingsPanel({ onSettingsChange }: SettingsPanelProps) {
  const { t } = useTranslation();
  const { theme, setTheme } = useTheme();
  const [inputDevices, setInputDevices] = useState<AudioDeviceInfo[]>([]);
  const [outputDevices, setOutputDevices] = useState<AudioDeviceInfo[]>([]);
  const [selectedInputId, setSelectedInputId] = useState<string | null>(null);
  const [selectedOutputId, setSelectedOutputId] = useState<string | null>(null);
  const [bufferSize, setBufferSize] = useState<number>(64);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [peerName, setPeerName] = useState<string>("User");
  const [peerNameError, setPeerNameError] = useState<string | null>(null);

  // Buffer size options: 8, 16, 32, 64, 128, 256
  const bufferSizeOptions = [8, 16, 32, 64, 128, 256];
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

        const [inputs, outputs, current, currentBufferSize, savedPeerName] =
          await Promise.all([
            audioListInputDevices(),
            audioListOutputDevices(),
            audioGetCurrentDevices(),
            audioGetBufferSize(),
            configGetPeerName().catch(() => "User"),
          ]);

        setPeerName(savedPeerName);

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
      await audioSetInputDevice(deviceId);
      setSelectedInputId(deviceId);
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
      await audioSetOutputDevice(deviceId);
      setSelectedOutputId(deviceId);
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

  // Helper to restart streaming with new buffer size
  const restartStreamingWithBufferSize = async (newBufferSize: number) => {
    try {
      const status = await streamingStatus();
      if (status.is_active && status.remote_addr) {
        await streamingStop();
        // Small delay to let audio resources release
        await new Promise(resolve => setTimeout(resolve, 100));
        await streamingStart(
          status.remote_addr,
          selectedInputId ?? undefined,
          selectedOutputId ?? undefined,
          newBufferSize
        );
      }
    } catch {
      // Ignore errors from streaming restart
    }
  };

  const handleBufferSizeChange = async (newSize: number) => {
    try {
      setError(null);
      await audioSetBufferSize(newSize);
      setBufferSize(newSize);
      await saveConfig(selectedInputId, selectedOutputId, newSize);

      // If streaming is active, restart with new buffer size
      await restartStreamingWithBufferSize(newSize);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  // Handle peer name change with debounce
  const handlePeerNameChange = useCallback(async (newName: string) => {
    setPeerName(newName);
    setPeerNameError(null);

    // Validate
    const trimmed = newName.trim();
    if (trimmed.length === 0) {
      setPeerNameError(t("settings.profile.nameRequired", "Name is required"));
      return;
    }
    if (trimmed.length > 32) {
      setPeerNameError(t("settings.profile.nameTooLong", "Name must be 32 characters or less"));
      return;
    }

    // Save to config
    try {
      await configSetPeerName(trimmed);
      onSettingsChange?.();
    } catch (err) {
      setPeerNameError(err instanceof Error ? err.message : String(err));
    }
  }, [t, onSettingsChange]);

  // Calculate latency in ms for display
  const calculateLatencyMs = (samples: number) =>
    ((samples / 48000) * 1000).toFixed(2);

  return (
    <div className="settings-panel">
      {/* Profile / User Name */}
      <section className="settings-section">
        <h3 className="settings-section__title">{t("settings.profile.title", "Profile")}</h3>

        <div className="settings-field">
          <label className="settings-field__label" htmlFor="peer-name">
            {t("settings.profile.name", "Display Name")}
          </label>
          <input
            id="peer-name"
            type="text"
            className={`settings-field__input ${peerNameError ? "settings-field__input--error" : ""}`}
            value={peerName}
            onChange={(e) => handlePeerNameChange(e.target.value)}
            placeholder="User"
            maxLength={32}
          />
          {peerNameError && (
            <p className="settings-field__error">{peerNameError}</p>
          )}
          <p className="settings-field__hint">
            {t("settings.profile.nameHint", "This name will be shown to other participants")}
          </p>
        </div>
      </section>

      {/* Audio Devices */}
      <section className="settings-section">
        <h3 className="settings-section__title">{t("settings.audio.devices")}</h3>

        {error && <div className="settings-error">{error}</div>}

        <div className="settings-section__devices">
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
      </section>

      {/* Buffer Size */}
      <section className="settings-section">
        <h3 className="settings-section__title">{t("settings.audio.buffer")}</h3>
        <p className="settings-section__hint">
          {t("settings.audio.bufferHint")}
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
            max="5"
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
              {t("settings.audio.bufferValue", { samples: bufferSize, ms: calculateLatencyMs(bufferSize) })}
            </span>
          </div>
        </div>
      </section>

      {/* Theme Selection */}
      <section className="settings-section">
        <h3 className="settings-section__title">{t("settings.display.title")}</h3>

        <div className="theme-selector">
          <button
            className={`theme-option ${theme === "dark" ? "theme-option--selected" : ""}`}
            onClick={() => setTheme("dark")}
          >
            <span className="theme-option__icon">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
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
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
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
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
                <line x1="8" y1="21" x2="16" y2="21" />
                <line x1="12" y1="17" x2="12" y2="21" />
              </svg>
            </span>
            <span className="theme-option__label">{t("settings.display.themeSystem")}</span>
          </button>
        </div>
      </section>
    </div>
  );
}

export default SettingsPanel;
