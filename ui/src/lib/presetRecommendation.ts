/**
 * Preset recommendation logic based on connection quality
 *
 * Recommends audio presets based on measured jitter values.
 * Based on docs-spec/architecture.md section 8.1 "Connection Quality Monitoring"
 */

/**
 * Audio preset identifier type (matches backend AudioPreset)
 */
export type AudioPresetId =
  | "zero-latency"
  | "ultra-low-latency"
  | "balanced"
  | "high-quality";

/**
 * Jitter thresholds for preset recommendations (in milliseconds)
 *
 * These thresholds are based on typical network conditions:
 * - Fiber connections (Japan domestic): < 1ms jitter
 * - LAN sessions: 1-3ms jitter
 * - Typical internet: 3-10ms jitter
 * - Unstable connections: > 10ms jitter
 */
export const JITTER_THRESHOLDS = {
  ZERO_LATENCY_MAX: 1, // < 1ms -> zero-latency
  ULTRA_LOW_LATENCY_MAX: 3, // 1-3ms -> ultra-low-latency
  BALANCED_MAX: 10, // 3-10ms -> balanced
  // > 10ms -> high-quality
} as const;

/**
 * Get recommended preset based on jitter value
 *
 * @param jitterMs Jitter in milliseconds
 * @returns Recommended preset ID, or null if jitter is invalid
 */
export function getRecommendedPreset(jitterMs: number): AudioPresetId | null {
  // Handle invalid values
  if (
    jitterMs === undefined ||
    jitterMs === null ||
    Number.isNaN(jitterMs) ||
    jitterMs < 0
  ) {
    return null;
  }

  if (jitterMs < JITTER_THRESHOLDS.ZERO_LATENCY_MAX) {
    return "zero-latency";
  }

  if (jitterMs < JITTER_THRESHOLDS.ULTRA_LOW_LATENCY_MAX) {
    return "ultra-low-latency";
  }

  if (jitterMs < JITTER_THRESHOLDS.BALANCED_MAX) {
    return "balanced";
  }

  return "high-quality";
}

/**
 * Check if a preset change is recommended
 *
 * @param currentPreset Current preset ID
 * @param jitterMs Jitter in milliseconds
 * @returns true if a different preset is recommended
 */
export function isPresetChangeRecommended(
  currentPreset: AudioPresetId,
  jitterMs: number
): boolean {
  const recommended = getRecommendedPreset(jitterMs);
  if (recommended === null) {
    return false;
  }
  return recommended !== currentPreset;
}

/**
 * Get human-readable description for a preset
 *
 * @param presetId Preset identifier
 * @returns Description object with name and details
 */
export function getPresetDescription(presetId: AudioPresetId): {
  name: string;
  description: string;
  useCase: string;
} {
  const descriptions: Record<
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

  return descriptions[presetId];
}
