/**
 * MuteButton component
 *
 * Provides a toggle button for muting/unmuting the microphone.
 */

import { useTranslation } from "react-i18next";
import "./MuteButton.css";

interface MuteButtonProps {
  /** Whether the microphone is currently muted */
  isMuted: boolean;
  /** Callback when mute state is toggled */
  onToggle: () => void;
  /** Whether the button is disabled */
  disabled?: boolean;
}

export function MuteButton({ isMuted, onToggle, disabled }: MuteButtonProps) {
  const { t } = useTranslation();

  return (
    <button
      className={`mute-button ${isMuted ? "mute-button--muted" : ""}`}
      onClick={onToggle}
      disabled={disabled}
      title={isMuted ? t("mute.unmute") : t("mute.mute")}
      aria-pressed={isMuted}
    >
      <span className="mute-button__icon">
        {isMuted ? (
          // Muted icon (microphone with line through)
          <svg
            width="24"
            height="24"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <line x1="1" y1="1" x2="23" y2="23" />
            <path d="M9 9v3a3 3 0 0 0 5.12 2.12M15 9.34V4a3 3 0 0 0-5.94-.6" />
            <path d="M17 16.95A7 7 0 0 1 5 12v-2m14 0v2a7 7 0 0 1-.11 1.23" />
            <line x1="12" y1="19" x2="12" y2="23" />
            <line x1="8" y1="23" x2="16" y2="23" />
          </svg>
        ) : (
          // Unmuted icon (microphone)
          <svg
            width="24"
            height="24"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z" />
            <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
            <line x1="12" y1="19" x2="12" y2="23" />
            <line x1="8" y1="23" x2="16" y2="23" />
          </svg>
        )}
      </span>
      <span className="mute-button__label">
        {isMuted ? t("mute.unmute") : t("mute.mute")}
      </span>
    </button>
  );
}
