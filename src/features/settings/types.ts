import type { Language } from "../../i18n";

/** How the launcher opens: with recent items, or just the search field. */
export type WindowMode = "expanded" | "compact";

/** UI font stack selected in the appearance settings. */
export type FontFamily = "inter" | "geist" | "system";

/** Theme choice stored in the settings (system follows Windows). */
export type ThemeMode = "system" | "light" | "dark";

/** Theme actually applied to the document. */
export type ResolvedTheme = "light" | "dark";

/** Everything the user can configure, persisted as one JSON object. */
export interface Settings {
  language: Language;
  shortcut: string;
  showRecent: boolean;
  maxRecent: number;
  fileSearch: boolean;
  clipboardHistory: boolean;
  launchAtStartup: boolean;
  windowMode: WindowMode;
  fontFamily: FontFamily;
  theme: ThemeMode;
  radius: number;
  shadow: number;
  fontSize: number;
}
