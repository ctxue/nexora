import type { ResultsMaxHeight, SearchWidth, Settings } from "./types";

export const RADIUS_MIN = 0;
export const RADIUS_MAX = 28;

export const SHADOW_MIN = 0;
export const SHADOW_MAX = 100;

export const FONT_SIZE_MIN = 11;
export const FONT_SIZE_MAX = 17;

export const FONT_SIZE_BASE = 13;

export const SEARCH_WIDTH_PRESETS: readonly SearchWidth[] = [
  500, 575, 650, 725, 800,
];

export const RESULTS_HEIGHT_PRESETS: readonly ResultsMaxHeight[] = [
  320, 400, 480,
];

export const SEARCH_PLACEHOLDER_MAX_LENGTH = 120;

export const DEFAULT_SETTINGS: Settings = {
  language: "en",
  shortcut: "Alt+Space",
  showRecent: true,
  maxRecent: 5,
  fileSearch: true,
  browserBookmarks: false,
  browserHistory: false,
  browserTabs: false,
  clipboardHistory: false,
  launchAtStartup: false,
  disableInFullscreen: false,
  launcherAnimation: true,
  windowMode: "compact",
  fontFamily: "system",
  theme: "system",
  radius: 18,
  shadow: 100,
  fontSize: 14,
  searchWidth: 650,
  resultsMaxHeight: 400,
  showSearchPlaceholder: false,
  searchPlaceholder: "",
};

export const RECENT_CHOICES = [3, 5, 8] as const;
