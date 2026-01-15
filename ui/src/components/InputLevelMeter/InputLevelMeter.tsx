/**
 * InputLevelMeter component
 *
 * Displays a VU-style meter showing the current audio input level.
 */

import { useTranslation } from "react-i18next";
import "./InputLevelMeter.css";

interface InputLevelMeterProps {
  /** Audio level from 0 to 100 */
  level: number;
  /** Whether the microphone is muted */
  isMuted?: boolean;
  /** Display variant */
  variant?: "default" | "mini";
}

export function InputLevelMeter({
  level,
  isMuted = false,
  variant = "default",
}: InputLevelMeterProps) {
  const { t } = useTranslation();

  // Clamp level between 0 and 100
  const clampedLevel = Math.max(0, Math.min(100, level));

  // Determine color based on level
  const getColorClass = () => {
    if (isMuted) return "input-level-meter__fill--muted";
    if (clampedLevel > 80) return "input-level-meter__fill--high";
    if (clampedLevel > 50) return "input-level-meter__fill--medium";
    return "input-level-meter__fill--low";
  };

  return (
    <div
      className={`input-level-meter input-level-meter--${variant}`}
      role="meter"
      aria-valuenow={clampedLevel}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-label={t("audioLevel.label")}
    >
      {variant === "default" && (
        <span className="input-level-meter__label">{t("audioLevel.label")}</span>
      )}
      <div className="input-level-meter__bar">
        <div
          className={`input-level-meter__fill ${getColorClass()}`}
          style={{ width: `${clampedLevel}%` }}
        />
        {/* Tick marks at 25%, 50%, 75% */}
        <div className="input-level-meter__ticks">
          <div className="input-level-meter__tick" style={{ left: "25%" }} />
          <div className="input-level-meter__tick" style={{ left: "50%" }} />
          <div className="input-level-meter__tick" style={{ left: "75%" }} />
        </div>
      </div>
      {variant === "default" && (
        <span className="input-level-meter__value">{clampedLevel}%</span>
      )}
    </div>
  );
}
