import type { Language } from "../../i18n";

export type WindowMode = "expanded" | "compact";

export type FontFamily = "inter" | "geist" | "system";

export type ThemeMode = "system" | "light" | "dark";

export type ResolvedTheme = "light" | "dark";

export type SearchWidth = 500 | 575 | 650 | 725 | 800;

export type ResultsMaxHeight = 320 | 400 | 480;

export interface Settings {
  language: Language;
  shortcut: string;
  showRecent: boolean;
  maxRecent: number;
  fileSearch: boolean;
  browserBookmarks: boolean;
  browserHistory: boolean;
  browserTabs: boolean;
  clipboardHistory: boolean;
  launchAtStartup: boolean;
  disableInFullscreen: boolean;
  launcherAnimation: boolean;
  windowMode: WindowMode;
  fontFamily: FontFamily;
  theme: ThemeMode;
  radius: number;
  shadow: number;
  fontSize: number;
  searchWidth: SearchWidth;
  resultsMaxHeight: ResultsMaxHeight;
  showSearchPlaceholder: boolean;
  searchPlaceholder: string;
}
