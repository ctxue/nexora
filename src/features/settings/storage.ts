import {
  DEFAULT_SETTINGS,
  FONT_SIZE_MAX,
  FONT_SIZE_MIN,
  RADIUS_MAX,
  RADIUS_MIN,
  RECENT_CHOICES,
  SHADOW_MAX,
  SHADOW_MIN,
} from "./defaults";
import type { Settings } from "./types";

/** localStorage key holding the whole configuration. */
export const SETTINGS_KEY = "nexora.settings.v1";

/** One-time marker for the 0.2.1 default-shadow change (70% -> 100%). */
const SHADOW_DEFAULT_MIGRATION_KEY = "nexora.settings.shadow-default.v1";

export function clampRadius(value: unknown): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return DEFAULT_SETTINGS.radius;
  }
  return Math.min(RADIUS_MAX, Math.max(RADIUS_MIN, Math.round(value)));
}

export function clampShadow(value: unknown): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return DEFAULT_SETTINGS.shadow;
  }
  return Math.min(SHADOW_MAX, Math.max(SHADOW_MIN, Math.round(value)));
}

export function clampFontSize(value: unknown): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return DEFAULT_SETTINGS.fontSize;
  }
  return Math.min(FONT_SIZE_MAX, Math.max(FONT_SIZE_MIN, Math.round(value)));
}

/** Coerces arbitrary data (stored or imported) into a complete settings object. */
export function normalizeSettings(parsed: unknown): Settings {
  if (!parsed || typeof parsed !== "object") return DEFAULT_SETTINGS;
  const candidate = parsed as Partial<Settings>;

  return {
    language: candidate.language === "zh" ? "zh" : "en",
    shortcut:
      typeof candidate.shortcut === "string" && candidate.shortcut.trim()
        ? candidate.shortcut
        : DEFAULT_SETTINGS.shortcut,
    showRecent:
      typeof candidate.showRecent === "boolean"
        ? candidate.showRecent
        : DEFAULT_SETTINGS.showRecent,
    maxRecent: RECENT_CHOICES.includes(candidate.maxRecent as 3 | 5 | 8)
      ? (candidate.maxRecent as number)
      : DEFAULT_SETTINGS.maxRecent,
    fileSearch:
      typeof candidate.fileSearch === "boolean"
        ? candidate.fileSearch
        : DEFAULT_SETTINGS.fileSearch,
    launchAtStartup:
      typeof candidate.launchAtStartup === "boolean"
        ? candidate.launchAtStartup
        : DEFAULT_SETTINGS.launchAtStartup,
    windowMode: candidate.windowMode === "compact" ? "compact" : "expanded",
    fontFamily:
      candidate.fontFamily === "geist" || candidate.fontFamily === "system"
        ? candidate.fontFamily
        : DEFAULT_SETTINGS.fontFamily,
    theme:
      candidate.theme === "light" || candidate.theme === "dark"
        ? candidate.theme
        : DEFAULT_SETTINGS.theme,
    radius: clampRadius(candidate.radius),
    shadow: clampShadow(candidate.shadow),
    fontSize: clampFontSize(candidate.fontSize),
  };
}

/** Parses the value stored in localStorage, falling back to defaults. */
export function parseSettings(raw: string): Settings {
  try {
    return normalizeSettings(JSON.parse(raw));
  } catch {
    return DEFAULT_SETTINGS;
  }
}

export function loadSettings(): Settings {
  try {
    const raw = window.localStorage.getItem(SETTINGS_KEY);
    if (!raw) return DEFAULT_SETTINGS;

    const settings = parseSettings(raw);

    // 0.2.1 raised the shipped shadow default from 70% to 100%. Older installs
    // stored 70% even when the user never touched the slider, so migrate that
    // value once; a deliberate 70% set afterwards is left alone.
    if (window.localStorage.getItem(SHADOW_DEFAULT_MIGRATION_KEY) !== "1") {
      window.localStorage.setItem(SHADOW_DEFAULT_MIGRATION_KEY, "1");
      if (settings.shadow === 70) {
        settings.shadow = 100;
        saveSettings(settings);
      }
    }

    return settings;
  } catch {
    return DEFAULT_SETTINGS;
  }
}

export function saveSettings(settings: Settings) {
  try {
    window.localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
  } catch {
    // Settings are a convenience; storage failure should not break the app.
  }
}
