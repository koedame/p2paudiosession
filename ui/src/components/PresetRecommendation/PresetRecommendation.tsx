/**
 * PresetRecommendation component
 *
 * Displays recommended audio preset based on current network jitter.
 * Shows a suggestion banner with a button to switch presets when
 * the current preset differs from the recommended one.
 */

import { useTranslation } from "react-i18next";
import {
  getRecommendedPreset,
  isPresetChangeRecommended,
  getPresetDescription,
  type AudioPresetId,
} from "../../lib/presetRecommendation";
import "./PresetRecommendation.css";

export interface PresetRecommendationProps {
  /** Current jitter value in milliseconds */
  jitterMs: number | null;
  /** Currently selected preset */
  currentPreset: AudioPresetId;
  /** Callback when user clicks to switch preset */
  onSwitchPreset: (presetId: AudioPresetId) => void;
  /** Whether switching is disabled (e.g., during loading) */
  disabled?: boolean;
}

export function PresetRecommendation({
  jitterMs,
  currentPreset,
  onSwitchPreset,
  disabled = false,
}: PresetRecommendationProps) {
  const { t } = useTranslation();

  // Don't render if no jitter data
  if (jitterMs === null || jitterMs === undefined) {
    return null;
  }

  const recommendedPreset = getRecommendedPreset(jitterMs);

  // Don't render if we can't determine recommendation
  if (recommendedPreset === null) {
    return null;
  }

  // Don't render if current preset matches recommendation
  if (!isPresetChangeRecommended(currentPreset, jitterMs)) {
    return null;
  }

  const recommendedDescription = getPresetDescription(recommendedPreset);
  const currentDescription = getPresetDescription(currentPreset);

  const handleSwitch = () => {
    if (!disabled) {
      onSwitchPreset(recommendedPreset);
    }
  };

  return (
    <div className="preset-recommendation">
      <div className="preset-recommendation__icon" aria-hidden="true">
        <svg
          width="20"
          height="20"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
        >
          <circle cx="12" cy="12" r="10" />
          <line x1="12" y1="8" x2="12" y2="12" />
          <line x1="12" y1="16" x2="12.01" y2="16" />
        </svg>
      </div>
      <div className="preset-recommendation__content">
        <p className="preset-recommendation__message">
          {t("preset.recommendation.message", {
            defaultValue:
              "Based on your connection ({{jitter}}ms jitter), we recommend switching to {{preset}}",
            jitter: jitterMs.toFixed(1),
            preset: recommendedDescription.name,
          })}
        </p>
        <p className="preset-recommendation__detail">
          {t("preset.recommendation.current", {
            defaultValue: "Current: {{preset}}",
            preset: currentDescription.name,
          })}
        </p>
      </div>
      <button
        className="preset-recommendation__button"
        onClick={handleSwitch}
        disabled={disabled}
      >
        {t("preset.recommendation.switch", "Switch")}
      </button>
    </div>
  );
}

export default PresetRecommendation;
