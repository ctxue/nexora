import type { Settings } from "./types";

export const RADIUS_MIN = 0;
export const RADIUS_MAX = 28;

export const SHADOW_MIN = 0;
export const SHADOW_MAX = 100;

export const FONT_SIZE_MIN = 11;
export const FONT_SIZE_MAX = 17;

export const FONT_SIZE_BASE = 13;

export const DEFAULT_SETTINGS: Settings = {
  language: "en",
  shortcut: "Alt+Space",
  showRecent: true,
  maxRecent: 5,
  fileSearch: true,
  clipboardHistory: false,
  launchAtStartup: false,
  launcherAnimation: true,
  windowMode: "compact",
  fontFamily: "system",
  theme: "system",
  radius: 8,
  shadow: 100,
  fontSize: 14,
};

export const RECENT_CHOICES = [3, 5, 8] as const;
