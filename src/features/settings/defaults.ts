import type { Settings } from "./types";

/** Launcher corner radius range, in pixels. */
export const RADIUS_MIN = 0;
export const RADIUS_MAX = 28;

/** Launcher drop shadow strength, as a percentage of the designed depth. */
export const SHADOW_MIN = 0;
export const SHADOW_MAX = 100;

/** Global text size range, in pixels. */
export const FONT_SIZE_MIN = 11;
export const FONT_SIZE_MAX = 17;

/** The stylesheet's px values were tuned against this text size. */
export const FONT_SIZE_BASE = 13;

export const DEFAULT_SETTINGS: Settings = {
  language: "en",
  shortcut: "Alt+Space",
  showRecent: true,
  maxRecent: 5,
  fileSearch: true,
  launchAtStartup: false,
  windowMode: "expanded",
  fontFamily: "system",
  theme: "system",
  radius: 8,
  shadow: 70,
  fontSize: 14,
};

/** Number of recent items offered in the settings. */
export const RECENT_CHOICES = [3, 5, 8] as const;
