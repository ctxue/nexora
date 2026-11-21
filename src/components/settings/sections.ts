import {
  Blocks,
  LayoutGrid,
  Monitor,
  Moon,
  Palette,
  Puzzle,
  Settings2,
  SquareTerminal,
  Sun,
  UserRound,
  type LucideIcon,
} from "lucide-react";

import type { Settings, ThemeMode } from "../../features/settings/types";
import { DEFAULT_SETTINGS } from "../../features/settings/defaults";
import type { SettingsSectionId } from "../../features/settings/navigation";

export type SectionId = SettingsSectionId;

export type NavId = SectionId | "plugins";

export interface SectionBase {
  labelKey: string;
  icon: LucideIcon;
  badgeKey?: string;
  badgeTitleKey?: string;
}

export interface SettingsSection extends SectionBase {
  id: SectionId;
  locked?: false;
}

export interface LockedSection extends SectionBase {
  id: NavId;
  locked: true;
  teaserKey: string;
}

export type SectionDefinition = SettingsSection | LockedSection;

export function isSettingsSection(
  section: SectionDefinition,
): section is SettingsSection {
  return !section.locked;
}

export function isLockedSection(
  section: SectionDefinition,
): section is LockedSection {
  return section.locked === true;
}

export const SECTIONS: SectionDefinition[] = [
  { id: "profile", labelKey: "settings.profile", icon: UserRound },
  { id: "general", labelKey: "settings.general", icon: Settings2 },
  { id: "appearance", labelKey: "settings.appearance", icon: Palette },
  { id: "apps", labelKey: "settings.apps", icon: LayoutGrid },
  { id: "commands", labelKey: "settings.commands", icon: SquareTerminal },
  { id: "features", labelKey: "settings.features", icon: Blocks },
  {
    id: "plugins",
    labelKey: "settings.plugins",
    icon: Puzzle,
    badgeKey: "settings.pluginsBadge",
    badgeTitleKey: "settings.pluginsBadgeHint",
  },
];

export const DEFAULT_SECTION: SectionId = "general";

export interface ThemeOption {
  value: ThemeMode;
  labelKey: string;
  icon: LucideIcon;
}

export const THEME_OPTIONS: ThemeOption[] = [
  { value: "system", labelKey: "settings.themeSystem", icon: Monitor },
  { value: "light", labelKey: "settings.themeLight", icon: Sun },
  { value: "dark", labelKey: "settings.themeDark", icon: Moon },
];

export const FONT_STACKS: Record<Settings["fontFamily"], string> = {
  inter:
    '"Inter Variable", "Inter", "Segoe UI Variable Text", "Microsoft YaHei UI", "PingFang SC", sans-serif',
  geist:
    '"Geist Variable", "Geist", "Segoe UI Variable Text", "Microsoft YaHei UI", "PingFang SC", sans-serif',
  system:
    '-apple-system, BlinkMacSystemFont, "Segoe UI Variable Text", "Segoe UI", "Microsoft YaHei UI", "PingFang SC", sans-serif',
};

export const FONT_OPTIONS: Array<{
  value: Settings["fontFamily"];
  labelKey: string;
}> = [
  { value: "inter", labelKey: "settings.fontInter" },
  { value: "geist", labelKey: "settings.fontGeist" },
  { value: "system", labelKey: "settings.fontSystem" },
];

export const SHORTCUT_PRESETS = [
  DEFAULT_SETTINGS.shortcut,
  "Ctrl+Space",
  "Ctrl+Alt+Space",
  "Shift+Space",
];
