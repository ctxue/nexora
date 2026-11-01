import { describe, expect, it } from "vitest";

import { DEFAULT_SETTINGS } from "./defaults";
import {
  clampFontSize,
  clampRadius,
  clampShadow,
  normalizeSettings,
} from "./storage";

describe("normalizeSettings", () => {
  it("returns defaults for non-object input", () => {
    expect(normalizeSettings(null)).toEqual(DEFAULT_SETTINGS);
  });

  it("keeps valid values while replacing invalid or missing values", () => {
    const settings = normalizeSettings({
      language: "zh",
      theme: "dark",
      fontSize: 16,
      fileSearch: false,
    });

    expect(settings.language).toBe("zh");
    expect(settings.theme).toBe("dark");
    expect(settings.fontSize).toBe(16);
    expect(settings.fileSearch).toBe(false);
    expect(settings.shortcut).toBe(DEFAULT_SETTINGS.shortcut);
  });
});

describe("appearance bounds", () => {
  it("clamps out-of-range values to the supported range", () => {
    expect(clampRadius(-10)).toBe(0);
    expect(clampRadius(99)).toBe(28);
    expect(clampShadow(-1)).toBe(0);
    expect(clampShadow(101)).toBe(100);
    expect(clampFontSize(8)).toBe(11);
    expect(clampFontSize(30)).toBe(17);
  });
});
