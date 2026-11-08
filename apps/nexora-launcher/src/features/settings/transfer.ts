import {
  normalizeCommands,
  type CustomCommand,
} from "../../core/commands";
import { normalizeSettings } from "./storage";
import type { Settings } from "./types";

const EXPORT_APP = "NEXORA";

const EXPORT_VERSION = 2;

const SETTINGS_FIELDS = [
  "language",
  "shortcut",
  "showRecent",
  "maxRecent",
  "fileSearch",
  "clipboardHistory",
  "launchAtStartup",
  "launcherAnimation",
  "windowMode",
  "fontFamily",
  "theme",
  "radius",
  "shadow",
  "fontSize",
] as const;

export interface LoadedConfiguration {
  settings: Settings;
  commands: CustomCommand[];
  hasCommands: boolean;
}

function looksLikeSettings(value: unknown): boolean {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const record = value as Record<string, unknown>;
  return SETTINGS_FIELDS.some((field) => field in record);
}

export function exportPayload(
  settings: Settings,
  commands: CustomCommand[],
) {
  return {
    app: EXPORT_APP,
    version: EXPORT_VERSION,
    exportedAt: new Date().toISOString(),
    settings,
    commands,
  };
}

export function settingsFromExport(text: string): LoadedConfiguration | null {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text.replace(/^\uFEFF/, ""));
  } catch {
    return null;
  }

  const wrapper =
    parsed && typeof parsed === "object" && !Array.isArray(parsed)
      ? (parsed as Record<string, unknown>)
      : null;

  const settingsCandidate =
    wrapper && "settings" in wrapper ? wrapper.settings : parsed;

  if (!looksLikeSettings(settingsCandidate)) return null;

  const hasCommands = Boolean(wrapper && Array.isArray(wrapper.commands));

  return {
    settings: normalizeSettings(settingsCandidate),
    commands: hasCommands ? normalizeCommands(wrapper?.commands) : [],
    hasCommands,
  };
}
