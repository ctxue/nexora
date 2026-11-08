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

export const SETTINGS_KEY = "nexora.settings.v1";

const SHADOW_DEFAULT_MIGRATION_KEY = "nexora.settings.shadow-default.v1";
const CLIPBOARD_PRIVACY_MIGRATION_KEY = "nexora.clipboard.privacy-pause.v1";
const COMPACT_DEFAULT_MIGRATION_KEY = "nexora.settings.compact-default.v1";

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
    clipboardHistory:
      typeof candidate.clipboardHistory === "boolean"
        ? candidate.clipboardHistory
        : DEFAULT_SETTINGS.clipboardHistory,
    launchAtStartup:
      typeof candidate.launchAtStartup === "boolean"
        ? candidate.launchAtStartup
        : DEFAULT_SETTINGS.launchAtStartup,
    launcherAnimation:
      typeof candidate.launcherAnimation === "boolean"
        ? candidate.launcherAnimation
        : DEFAULT_SETTINGS.launcherAnimation,
    windowMode:
      candidate.windowMode === "compact" || candidate.windowMode === "expanded"
        ? candidate.windowMode
        : DEFAULT_SETTINGS.windowMode,
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
    const settings = raw ? parseSettings(raw) : { ...DEFAULT_SETTINGS };
    let changed = false;

    if (window.localStorage.getItem(SHADOW_DEFAULT_MIGRATION_KEY) !== "1") {
      window.localStorage.setItem(SHADOW_DEFAULT_MIGRATION_KEY, "1");
      if (settings.shadow === 70) {
        settings.shadow = 100;
        changed = true;
      }
    }
    if (window.localStorage.getItem(CLIPBOARD_PRIVACY_MIGRATION_KEY) !== "1") {
      window.localStorage.setItem(CLIPBOARD_PRIVACY_MIGRATION_KEY, "1");
      if (settings.clipboardHistory) {
        settings.clipboardHistory = false;
        changed = true;
      }
    }
    if (window.localStorage.getItem(COMPACT_DEFAULT_MIGRATION_KEY) !== "1") {
      window.localStorage.setItem(COMPACT_DEFAULT_MIGRATION_KEY, "1");
      if (settings.windowMode === "expanded") {
        settings.windowMode = "compact";
        changed = true;
      }
    }

    if (changed) saveSettings(settings);

    return settings;
  } catch {
    return DEFAULT_SETTINGS;
  }
}

export function saveSettings(settings: Settings) {
  try {
    window.localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
  } catch {
  }
}
