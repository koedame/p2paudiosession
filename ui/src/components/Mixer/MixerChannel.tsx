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
  isSoloed: _isSoloed = false,
  color,
  onVolumeChange,
  onPanChange,
  onMuteToggle,
  onSoloToggle: _onSoloToggle,
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

  return (
    <div className={`mixer-channel mixer-channel--${type}`}>
      {/* Pan Slider */}
      <div className="mixer-channel__pan">
        <div className="mixer-channel__pan-label">
          {pan < -10 ? "L" : pan > 10 ? "R" : "C"}
          {Math.abs(pan) > 10 && Math.abs(pan) < 100 ? Math.abs(Math.round(pan / 10)) : ""}
        </div>
        <div className="mixer-channel__pan-slider-container" onDoubleClick={handlePanDoubleClick}>
          <input
            type="range"
            min="-100"
            max="100"
            value={pan}
            onChange={handlePanChange}
            className="mixer-channel__pan-slider"
            aria-label={`${name} pan`}
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

        {/* Level Meter */}
        <VerticalMeter level={level} isMuted={isMuted} height={200} />
      </div>

      {/* Mute Button (not shown for master channel) */}
      {type !== "master" && (
        <div className="mixer-channel__buttons">
          <button
            className={`mixer-channel__btn mixer-channel__btn--mute ${!isMuted ? "mixer-channel__btn--active" : ""}`}
            onClick={handleMuteClick}
            aria-label={`${name} ${type === "input" ? "mic" : "mute"}`}
            aria-pressed={isMuted}
          >
            {type === "input" ? (
              isMuted ? (
                // Microphone off icon (with slash)
                <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M19 11h-1.7c0 .74-.16 1.43-.43 2.05l1.23 1.23c.56-.98.9-2.09.9-3.28zm-4.02.17c0-.06.02-.11.02-.17V5c0-1.66-1.34-3-3-3S9 3.34 9 5v.18l5.98 5.99zM4.27 3L3 4.27l6.01 6.01V11c0 1.66 1.33 3 2.99 3 .22 0 .44-.03.65-.08l1.66 1.66c-.71.33-1.5.52-2.31.52-2.76 0-5.3-2.1-5.3-5.1H5c0 3.41 2.72 6.23 6 6.72V21h2v-3.28c.91-.13 1.77-.45 2.54-.9L19.73 21 21 19.73 4.27 3z"/>
                </svg>
              ) : (
                // Microphone on icon
                <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M12 14c1.66 0 3-1.34 3-3V5c0-1.66-1.34-3-3-3S9 3.34 9 5v6c0 1.66 1.34 3 3 3z"/>
                  <path d="M17 11c0 2.76-2.24 5-5 5s-5-2.24-5-5H5c0 3.53 2.61 6.43 6 6.92V21h2v-3.08c3.39-.49 6-3.39 6-6.92h-2z"/>
                </svg>
              )
            ) : (
              isMuted ? (
                // Speaker off icon (with slash)
                <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M16.5 12c0-1.77-1.02-3.29-2.5-4.03v2.21l2.45 2.45c.03-.2.05-.41.05-.63zm2.5 0c0 .94-.2 1.82-.54 2.64l1.51 1.51C20.63 14.91 21 13.5 21 12c0-4.28-2.99-7.86-7-8.77v2.06c2.89.86 5 3.54 5 6.71zM4.27 3L3 4.27 7.73 9H3v6h4l5 5v-6.73l4.25 4.25c-.67.52-1.42.93-2.25 1.18v2.06c1.38-.31 2.63-.95 3.69-1.81L19.73 21 21 19.73l-9-9L4.27 3zM12 4L9.91 6.09 12 8.18V4z"/>
                </svg>
              ) : (
                // Speaker on icon
                <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M3 9v6h4l5 5V4L7 9H3zm13.5 3c0-1.77-1.02-3.29-2.5-4.03v8.05c1.48-.73 2.5-2.25 2.5-4.02zM14 3.23v2.06c2.89.86 5 3.54 5 6.71s-2.11 5.85-5 6.71v2.06c4.01-.91 7-4.49 7-8.77s-2.99-7.86-7-8.77z"/>
                </svg>
              )
            )}
          </button>
        </div>
      )}

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
