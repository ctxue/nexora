import { afterEach, describe, expect, it, vi } from "vitest";

import { DEFAULT_SETTINGS, SEARCH_WIDTH_PRESETS } from "./defaults";
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

  it("keeps browser data sources disabled by default and preserves opt-ins", () => {
    expect(normalizeSettings({})).toMatchObject({
      browserBookmarks: false,
      browserHistory: false,
      browserTabs: false,
    });
    expect(
      normalizeSettings({
        browserBookmarks: true,
        browserHistory: true,
        browserTabs: true,
      }),
    ).toMatchObject({
      browserBookmarks: true,
      browserHistory: true,
      browserTabs: true,
    });
  });

  it("keeps the fullscreen shortcut guard opt-in", () => {
    expect(normalizeSettings({})).toMatchObject({
      disableInFullscreen: false,
    });
    expect(normalizeSettings({ disableInFullscreen: true })).toMatchObject({
      disableInFullscreen: true,
    });
  });

  it("defaults to the entrance animation and a collapsed launcher", () => {
    const settings = normalizeSettings({});

    expect(settings.launcherAnimation).toBe(true);
    expect(settings.windowMode).toBe("compact");
    expect(settings.radius).toBe(18);
    expect(settings.searchWidth).toBe(650);
    expect(settings).toMatchObject({ resultsMaxHeight: 400 });
    expect(settings.showSearchPlaceholder).toBe(false);
    expect(settings.searchPlaceholder).toBe("");
  });

  it("keeps an explicitly disabled entrance animation", () => {
    const settings = normalizeSettings({ launcherAnimation: false });

    expect(settings.launcherAnimation).toBe(false);
  });

  it("keeps the supported search width presets", () => {
    expect(SEARCH_WIDTH_PRESETS).toEqual([500, 575, 650, 725, 800]);
    for (const searchWidth of SEARCH_WIDTH_PRESETS) {
      expect(normalizeSettings({ searchWidth }).searchWidth).toBe(searchWidth);
    }
  });

  it("moves legacy search widths to the nearest preset", () => {
    expect(normalizeSettings({ searchWidth: 499 }).searchWidth).toBe(500);
    expect(normalizeSettings({ searchWidth: 1001 }).searchWidth).toBe(800);
    expect(normalizeSettings({ searchWidth: 720.6 }).searchWidth).toBe(725);
    expect(normalizeSettings({ searchWidth: 900 }).searchWidth).toBe(800);
    expect(normalizeSettings({ searchWidth: Number.NaN }).searchWidth).toBe(650);
  });

  it("moves results heights to the nearest supported preset", () => {
    expect(normalizeSettings({ resultsMaxHeight: 300 })).toMatchObject({
      resultsMaxHeight: 320,
    });
    expect(normalizeSettings({ resultsMaxHeight: 390 })).toMatchObject({
      resultsMaxHeight: 400,
    });
    expect(normalizeSettings({ resultsMaxHeight: 470 })).toMatchObject({
      resultsMaxHeight: 480,
    });
    expect(normalizeSettings({ resultsMaxHeight: 900 })).toMatchObject({
      resultsMaxHeight: 480,
    });
    expect(normalizeSettings({ resultsMaxHeight: Number.NaN })).toMatchObject({
      resultsMaxHeight: 400,
    });
  });

  it("normalizes the optional search placeholder", () => {
    expect(
      normalizeSettings({
        showSearchPlaceholder: true,
        searchPlaceholder: "  Search now  ",
      }),
    ).toMatchObject({
      showSearchPlaceholder: true,
      searchPlaceholder: "Search now",
    });

    expect(
      normalizeSettings({
        showSearchPlaceholder: "yes",
        searchPlaceholder: "x".repeat(200),
      }),
    ).toMatchObject({
      showSearchPlaceholder: false,
      searchPlaceholder: "x".repeat(120),
    });
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

describe("loadSettings radius default upgrade", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("upgrades an old 8px radius once without overriding later choices", () => {
    const values = new Map<string, string>();
    vi.stubGlobal("window", {
      localStorage: {
        getItem: (key: string) => values.get(key) ?? null,
        setItem: (key: string, value: string) => values.set(key, value),
      },
    });
    saveSettings({ ...DEFAULT_SETTINGS, radius: 8 });

    expect(loadSettings().radius).toBe(18);
    expect(JSON.parse(values.get(SETTINGS_KEY) ?? "{}").radius).toBe(18);

    saveSettings({ ...DEFAULT_SETTINGS, radius: 8 });
    expect(loadSettings().radius).toBe(8);
  });

  it("preserves a custom radius during the default upgrade", () => {
    const values = new Map<string, string>();
    vi.stubGlobal("window", {
      localStorage: {
        getItem: (key: string) => values.get(key) ?? null,
        setItem: (key: string, value: string) => values.set(key, value),
      },
    });
    saveSettings({ ...DEFAULT_SETTINGS, radius: 12 });

    expect(loadSettings().radius).toBe(12);
  });
});
