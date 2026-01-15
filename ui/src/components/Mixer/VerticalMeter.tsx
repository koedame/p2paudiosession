/**
 * VerticalMeter - DAW-style vertical level meter
 *
 * Displays audio level with green/yellow/red gradient segments
 * like professional mixing consoles (Ableton, Logic Pro, etc.)
 */

import "./VerticalMeter.css";

interface VerticalMeterProps {
  /** Audio level from 0 to 100 */
  level: number;
  /** Peak hold level (optional) */
  peakLevel?: number;
  /** Whether the channel is muted */
  isMuted?: boolean;
  /** Height of the meter in pixels */
  height?: number;
}

// dB scale marks (visual reference)
const DB_MARKS = [
  { db: 6, position: 0 },
  { db: 3, position: 10 },
  { db: 0, position: 20 },
  { db: -3, position: 30 },
  { db: -6, position: 40 },
  { db: -12, position: 55 },
  { db: -18, position: 70 },
  { db: -24, position: 85 },
  { db: -48, position: 100 },
];

export function VerticalMeter({
  level,
  peakLevel,
  isMuted = false,
  height = 200,
}: VerticalMeterProps) {
  // Clamp level between 0 and 100
  const clampedLevel = Math.max(0, Math.min(100, level));
  const clampedPeak = peakLevel !== undefined ? Math.max(0, Math.min(100, peakLevel)) : undefined;

  // Calculate fill height percentage (inverted for bottom-up)
  const fillHeight = clampedLevel;
  const peakPosition = clampedPeak !== undefined ? 100 - clampedPeak : undefined;

  return (
    <div
      className={`vertical-meter ${isMuted ? "vertical-meter--muted" : ""}`}
      style={{ height }}
      role="meter"
      aria-valuenow={clampedLevel}
      aria-valuemin={0}
      aria-valuemax={100}
    >
      {/* dB scale marks */}
      <div className="vertical-meter__scale">
        {DB_MARKS.map(({ db, position }) => (
          <div
            key={db}
            className="vertical-meter__mark"
            style={{ top: `${position}%` }}
          >
            <span className="vertical-meter__mark-label">{db}</span>
            <span className="vertical-meter__mark-line" />
          </div>
        ))}
      </div>

      {/* Meter bar */}
      <div className="vertical-meter__bar">
        {/* Background segments for reference */}
        <div className="vertical-meter__segments">
          <div className="vertical-meter__segment vertical-meter__segment--red" />
          <div className="vertical-meter__segment vertical-meter__segment--yellow" />
          <div className="vertical-meter__segment vertical-meter__segment--green" />
        </div>

        {/* Active fill */}
        <div
          className="vertical-meter__fill"
          style={{ height: `${fillHeight}%` }}
        />

        {/* Peak indicator */}
        {peakPosition !== undefined && (
          <div
            className="vertical-meter__peak"
            style={{ top: `${peakPosition}%` }}
          />
        )}
      </div>
    </div>
  );
}

export default VerticalMeter;
