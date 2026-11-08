export type SettingsSectionId =
  | "profile"
  | "general"
  | "appearance"
  | "apps"
  | "commands"
  | "plugins"
  | "features";

export const SETTINGS_SECTION_REQUEST_KEY = "nexora.settings.open-section.v1";

const SETTINGS_SECTIONS: SettingsSectionId[] = [
  "profile",
  "general",
  "appearance",
  "apps",
  "commands",
  "plugins",
  "features",
];

function isSettingsSectionId(value: string): value is SettingsSectionId {
  return SETTINGS_SECTIONS.includes(value as SettingsSectionId);
}

export function requestSettingsSection(section: SettingsSectionId | null) {
  try {
    if (section) {
      window.localStorage.setItem(SETTINGS_SECTION_REQUEST_KEY, section);
    } else {
      window.localStorage.removeItem(SETTINGS_SECTION_REQUEST_KEY);
    }
  } catch {
    // Navigation is a convenience; the settings window still opens normally.
  }
}

export function readRequestedSettingsSection(): SettingsSectionId | null {
  try {
    const value = window.localStorage.getItem(SETTINGS_SECTION_REQUEST_KEY);
    if (!value) return null;
    window.localStorage.removeItem(SETTINGS_SECTION_REQUEST_KEY);
    return isSettingsSectionId(value) ? value : null;
  } catch {
    return null;
  }
}
