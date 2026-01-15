/**
 * Preset recommendation logic tests
 *
 * Based on docs-spec/architecture.md section 8.1 "Connection Quality Monitoring"
 * and Plans.md Phase 2 requirements.
 *
 * Jitter thresholds:
 * - < 1ms: zero-latency recommended
 * - 1-3ms: ultra-low-latency recommended
 * - 3-10ms: balanced recommended
 * - > 10ms: high-quality recommended
 */

import { describe, it, expect } from "vitest";
import {
  getRecommendedPreset,
  isPresetChangeRecommended,
} from "./presetRecommendation";

describe("getRecommendedPreset", () => {
  describe("zero-latency recommendation (jitter < 1ms)", () => {
    it("returns zero-latency for jitter 0ms", () => {
      expect(getRecommendedPreset(0)).toBe("zero-latency");
    });

    it("returns zero-latency for jitter 0.5ms", () => {
      expect(getRecommendedPreset(0.5)).toBe("zero-latency");
    });

    it("returns zero-latency for jitter 0.99ms", () => {
      expect(getRecommendedPreset(0.99)).toBe("zero-latency");
    });
  });

  describe("ultra-low-latency recommendation (1ms <= jitter < 3ms)", () => {
    it("returns ultra-low-latency for jitter 1ms", () => {
      expect(getRecommendedPreset(1)).toBe("ultra-low-latency");
    });

    it("returns ultra-low-latency for jitter 2ms", () => {
      expect(getRecommendedPreset(2)).toBe("ultra-low-latency");
    });

    it("returns ultra-low-latency for jitter 2.99ms", () => {
      expect(getRecommendedPreset(2.99)).toBe("ultra-low-latency");
    });
  });

  describe("balanced recommendation (3ms <= jitter < 10ms)", () => {
    it("returns balanced for jitter 3ms", () => {
      expect(getRecommendedPreset(3)).toBe("balanced");
    });

    it("returns balanced for jitter 5ms", () => {
      expect(getRecommendedPreset(5)).toBe("balanced");
    });

    it("returns balanced for jitter 9.99ms", () => {
      expect(getRecommendedPreset(9.99)).toBe("balanced");
    });
  });

  describe("high-quality recommendation (jitter >= 10ms)", () => {
    it("returns high-quality for jitter 10ms", () => {
      expect(getRecommendedPreset(10)).toBe("high-quality");
    });

    it("returns high-quality for jitter 15ms", () => {
      expect(getRecommendedPreset(15)).toBe("high-quality");
    });

    it("returns high-quality for jitter 100ms", () => {
      expect(getRecommendedPreset(100)).toBe("high-quality");
    });
  });

  describe("edge cases", () => {
    it("returns null for negative jitter", () => {
      expect(getRecommendedPreset(-1)).toBeNull();
    });

    it("returns null for NaN jitter", () => {
      expect(getRecommendedPreset(NaN)).toBeNull();
    });

    it("returns null for undefined jitter", () => {
      expect(getRecommendedPreset(undefined as unknown as number)).toBeNull();
    });
  });
});

describe("isPresetChangeRecommended", () => {
  it("returns false when current preset matches recommendation", () => {
    expect(isPresetChangeRecommended("zero-latency", 0.5)).toBe(false);
    expect(isPresetChangeRecommended("ultra-low-latency", 2)).toBe(false);
    expect(isPresetChangeRecommended("balanced", 5)).toBe(false);
    expect(isPresetChangeRecommended("high-quality", 15)).toBe(false);
  });

  it("returns true when current preset differs from recommendation", () => {
    expect(isPresetChangeRecommended("balanced", 0.5)).toBe(true);
    expect(isPresetChangeRecommended("high-quality", 2)).toBe(true);
    expect(isPresetChangeRecommended("zero-latency", 5)).toBe(true);
    expect(isPresetChangeRecommended("balanced", 15)).toBe(true);
  });

  it("returns false for invalid jitter values", () => {
    expect(isPresetChangeRecommended("balanced", -1)).toBe(false);
    expect(isPresetChangeRecommended("balanced", NaN)).toBe(false);
  });
});
