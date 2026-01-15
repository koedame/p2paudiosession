/**
 * MixerChannel - Single channel strip for the mixer console
 *
 * Includes: pan knob, dB display, level meter, fader, mute/solo buttons, channel label
 */

import { useState, useCallback } from "react";
import { VerticalMeter } from "./VerticalMeter";
import "./MixerChannel.css";

export type ChannelType = "input" | "peer" | "master";

export interface MixerChannelProps {
  /** Channel identifier */
  id: string;
  /** Display name */
  name: string;
  /** Channel type */
  type: ChannelType;
  /** Current audio level (0-100) */
  level: number;
  /** Volume/gain value (0-100, default 80 = 0dB) */
  volume: number;
  /** Pan value (-100 to 100, 0 = center) */
  pan?: number;
  /** Whether channel is muted */
  isMuted: boolean;
  /** Whether channel is soloed (only applies to peer channels) */
  isSoloed?: boolean;
  /** Channel color (CSS color value) */
  color?: string;
  /** Callback when volume changes */
  onVolumeChange?: (id: string, volume: number) => void;
  /** Callback when pan changes */
  onPanChange?: (id: string, pan: number) => void;
  /** Callback when mute is toggled */
  onMuteToggle?: (id: string) => void;
  /** Callback when solo is toggled */
  onSoloToggle?: (id: string) => void;
}

// Convert volume (0-100) to dB display
function volumeToDb(volume: number): number {
  if (volume === 0) return -Infinity;
  // 80 = 0dB, logarithmic scale
  const db = 20 * Math.log10(volume / 80);
  return Math.round(db * 10) / 10;
}

export function MixerChannel({
  id,
  name,
  type,
  level,
  volume,
  pan = 0,
  isMuted,
  isSoloed = false,
  color,
  onVolumeChange,
  onPanChange,
  onMuteToggle,
  onSoloToggle,
}: MixerChannelProps) {
  const [isDraggingFader, setIsDraggingFader] = useState(false);

  const handleVolumeChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const newVolume = parseInt(e.target.value, 10);
      onVolumeChange?.(id, newVolume);
    },
    [id, onVolumeChange]
  );

  const handlePanChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const newPan = parseInt(e.target.value, 10);
      onPanChange?.(id, newPan);
    },
    [id, onPanChange]
  );

  const handleMuteClick = useCallback(() => {
    onMuteToggle?.(id);
  }, [id, onMuteToggle]);

  const handleSoloClick = useCallback(() => {
    onSoloToggle?.(id);
  }, [id, onSoloToggle]);

  // Double-click to reset fader to 0dB (volume = 80)
  const handleFaderDoubleClick = useCallback(() => {
    onVolumeChange?.(id, 80);
  }, [id, onVolumeChange]);

  // Double-click to reset pan to center (pan = 0)
  const handlePanDoubleClick = useCallback(() => {
    onPanChange?.(id, 0);
  }, [id, onPanChange]);

  const dbValue = volumeToDb(volume);
  const dbDisplay = dbValue === -Infinity ? "-∞" : dbValue.toFixed(1);

  // Default colors by type
  const channelColor = color || (type === "input" ? "#ff6b35" : type === "master" ? "#7c5cff" : "#4ecdc4");

  // dB scale values for fader (like DAW mixers)
  const dbScaleValues = [0, 3, 6, 9, 12, 15, 18, 21, 24, 30, 35, 40, 45, 50, 60];

  return (
    <div className={`mixer-channel mixer-channel--${type}`}>
      {/* Pan Knob */}
      <div className="mixer-channel__pan">
        <div className="mixer-channel__knob-container" onDoubleClick={handlePanDoubleClick}>
          <input
            type="range"
            min="-100"
            max="100"
            value={pan}
            onChange={handlePanChange}
            className="mixer-channel__knob"
            aria-label={`${name} pan`}
          />
          <div
            className="mixer-channel__knob-indicator"
            style={{ transform: `rotate(${pan * 1.35}deg)` }}
          />
        </div>
      </div>

      {/* dB Display */}
      <div className="mixer-channel__db">
        <span className="mixer-channel__db-value">{dbDisplay}</span>
        <span className="mixer-channel__db-peak">{level > 0 ? `-${Math.round((100 - level) / 5)}` : "-∞"}</span>
      </div>

      {/* Meter and Fader Section */}
      <div className="mixer-channel__controls">
        {/* Fader with tick marks and scale */}
        <div className="mixer-channel__fader-section">
          {/* Tick marks (left side) */}
          <div className="mixer-channel__fader-ticks">
            {dbScaleValues.map((db, i) => (
              <div
                key={db}
                className={`mixer-channel__tick ${i % 3 === 0 ? "mixer-channel__tick--major" : ""}`}
              />
            ))}
          </div>

          {/* Fader */}
          <div className="mixer-channel__fader-container" onDoubleClick={handleFaderDoubleClick}>
            <div className="mixer-channel__fader-track" />
            <input
              type="range"
              min="0"
              max="100"
              value={volume}
              onChange={handleVolumeChange}
              onMouseDown={() => setIsDraggingFader(true)}
              onMouseUp={() => setIsDraggingFader(false)}
              className={`mixer-channel__fader ${isDraggingFader ? "mixer-channel__fader--dragging" : ""}`}
              aria-label={`${name} volume`}
            />
          </div>

          {/* dB Scale (right side) */}
          <div className="mixer-channel__db-scale">
            {dbScaleValues.map((db) => (
              <span key={db}>{db}</span>
            ))}
          </div>
        </div>

        {/* Level Meter */}
        <VerticalMeter level={level} isMuted={isMuted} height={200} />
      </div>

      {/* Mute / Solo Buttons */}
      <div className="mixer-channel__buttons">
        <button
          className={`mixer-channel__btn mixer-channel__btn--mute ${isMuted ? "mixer-channel__btn--active" : ""}`}
          onClick={handleMuteClick}
          aria-label={`${name} mute`}
          aria-pressed={isMuted}
        >
          M
        </button>
        {type !== "master" && (
          <button
            className={`mixer-channel__btn mixer-channel__btn--solo ${isSoloed ? "mixer-channel__btn--active" : ""}`}
            onClick={handleSoloClick}
            aria-label={`${name} solo`}
            aria-pressed={isSoloed}
          >
            S
          </button>
        )}
      </div>

      {/* Channel Label */}
      <div
        className="mixer-channel__label"
        style={{ backgroundColor: channelColor }}
      >
        {name}
      </div>
    </div>
  );
}

export default MixerChannel;
