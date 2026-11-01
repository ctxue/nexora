import {
  normalizeCommands,
  type CustomCommand,
} from "../../core/commands";
import { normalizeSettings } from "./storage";
import type { Settings } from "./types";

/** Marker written into exported files so they can be recognized later. */
const EXPORT_APP = "NEXORA";

/** v2 added the `commands` array next to `settings`. */
const EXPORT_VERSION = 2;

const SETTINGS_FIELDS = [
  "language",
  "shortcut",
  "showRecent",
  "maxRecent",
  "fileSearch",
  "clipboardHistory",
  "launchAtStartup",
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
  /** False for v1 files, which must not wipe the existing command list. */
  hasCommands: boolean;
}

/** A file counts as a NEXORA export when it carries at least one of our keys. */
function looksLikeSettings(value: unknown): boolean {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const record = value as Record<string, unknown>;
  return SETTINGS_FIELDS.some((field) => field in record);
}

/** The JSON document written by "export configuration". */
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

/**
 * Reads an exported configuration file. Accepts both the wrapper written by
 * `exportPayload` and a bare settings object (v0.1 files).
 */
export function settingsFromExport(text: string): LoadedConfiguration | null {
  let parsed: unknown;
  try {
    // Notepad and PowerShell may save a UTF-8 BOM; JSON.parse rejects it.
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
