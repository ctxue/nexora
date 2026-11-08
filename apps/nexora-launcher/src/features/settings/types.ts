import type { Language } from "../../i18n";

export type WindowMode = "expanded" | "compact";

export type FontFamily = "inter" | "geist" | "system";

export type ThemeMode = "system" | "light" | "dark";

export type ResolvedTheme = "light" | "dark";

export interface Settings {
  language: Language;
  shortcut: string;
  showRecent: boolean;
  maxRecent: number;
  fileSearch: boolean;
  clipboardHistory: boolean;
  launchAtStartup: boolean;
  launcherAnimation: boolean;
  windowMode: WindowMode;
  fontFamily: FontFamily;
  theme: ThemeMode;
  radius: number;
  shadow: number;
  fontSize: number;
}
