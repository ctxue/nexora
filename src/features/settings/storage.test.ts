import { afterEach, describe, expect, it, vi } from "vitest";

import { DEFAULT_SETTINGS } from "./defaults";
import {
  clampFontSize,
  clampRadius,
  clampShadow,
  loadSettings,
  normalizeSettings,
  saveSettings,
  SETTINGS_KEY,
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

  it("keeps clipboard history paused by default", () => {
    expect(normalizeSettings({}).clipboardHistory).toBe(false);
  });

  it("defaults to the entrance animation and a collapsed launcher", () => {
    const settings = normalizeSettings({});

    expect(settings.launcherAnimation).toBe(true);
    expect(settings.windowMode).toBe("compact");
  });

  it("keeps an explicitly disabled entrance animation", () => {
    const settings = normalizeSettings({ launcherAnimation: false });

    expect(settings.launcherAnimation).toBe(false);
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

describe("loadSettings clipboard privacy upgrade", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("pauses a previously enabled clipboard once so the user can review it", () => {
    const values = new Map<string, string>();
    vi.stubGlobal("window", {
      localStorage: {
        getItem: (key: string) => values.get(key) ?? null,
        setItem: (key: string, value: string) => values.set(key, value),
      },
    });
    saveSettings({ ...DEFAULT_SETTINGS, clipboardHistory: true });

    expect(loadSettings().clipboardHistory).toBe(false);
    expect(JSON.parse(values.get(SETTINGS_KEY) ?? "{}").clipboardHistory).toBe(
      false,
    );

    saveSettings({ ...DEFAULT_SETTINGS, clipboardHistory: true });
    expect(loadSettings().clipboardHistory).toBe(true);
  });
});

describe("loadSettings compact launcher upgrade", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("opens old stored configurations collapsed once", () => {
    const values = new Map<string, string>();
    vi.stubGlobal("window", {
      localStorage: {
        getItem: (key: string) => values.get(key) ?? null,
        setItem: (key: string, value: string) => values.set(key, value),
      },
    });
    saveSettings({ ...DEFAULT_SETTINGS, windowMode: "expanded" });

    expect(loadSettings().windowMode).toBe("compact");
    expect(JSON.parse(values.get(SETTINGS_KEY) ?? "{}").windowMode).toBe(
      "compact",
    );

    saveSettings({ ...DEFAULT_SETTINGS, windowMode: "expanded" });
    expect(loadSettings().windowMode).toBe("expanded");
  });
});
