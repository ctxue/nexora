import {
  AlertCircle,
  Download,
  Keyboard,
  RotateCcw,
  Trash2,
  Upload,
  UserRound,
} from "lucide-react";
import { useEffect, useState } from "react";

import {
  FONT_SIZE_MAX,
  FONT_SIZE_MIN,
  RADIUS_MAX,
  RADIUS_MIN,
  SHADOW_MAX,
  SHADOW_MIN,
} from "../features/settings/defaults";
import type { Settings } from "../features/settings/types";
import type { CustomCommand } from "../core/commands";
import { appNameFromPath, type ManualApp } from "../features/apps/storage";
import { platform, type PluginManifest } from "../platform/tauri";
import { useI18n } from "../i18n";
import { Segmented, ShortcutKeys, SliderRow, Toggle } from "./settings/Controls";
import { AppsSettings } from "./settings/AppsSettings";
import { CommandsSettings } from "./settings/CommandsSettings";
import { FeaturesSettings } from "./settings/FeaturesSettings";
import { PluginsSettings } from "./settings/PluginsSettings";
import {
  DEFAULT_SECTION,
  FONT_OPTIONS,
  FONT_STACKS,
  isLockedSection,
  isSettingsSection,
  SECTIONS,
  SHORTCUT_PRESETS,
  THEME_OPTIONS,
  type SectionId,
} from "./settings/sections";
import { useConfigurationTransfer } from "./settings/useConfigurationTransfer";
import { useNotice } from "./settings/useNotice";
import { useShortcutRecorder } from "./settings/useShortcutRecorder";
import {
  readRequestedSettingsSection,
  SETTINGS_SECTION_REQUEST_KEY,
} from "../features/settings/navigation";

interface SettingsPanelProps {
  settings: Settings;
  onChange: (patch: Partial<Settings>) => void;
  onShortcutChange: (shortcut: string) => Promise<boolean>;
  shortcutConflict: boolean;
  activeShortcut: string | null;
  onLaunchAtStartupChange: (enabled: boolean) => Promise<boolean>;
  onClearRecents: () => void;
  onResetSettings: () => void;
  onResetPosition: () => void;
  commands: CustomCommand[];
  onCommandsChange: (commands: CustomCommand[]) => void;
  manualApps: ManualApp[];
  onAddManualApps: (apps: Array<{ path: string; name: string }>) => void;
  onRemoveManualApp: (id: string) => void;
  indexFolders: Array<{ id: string; path: string }>;
  onAddIndexFolders: (paths: string[]) => void;
  onRemoveIndexFolder: (id: string) => void;
  onRefreshIndexes: () => void;
  plugins: PluginManifest[];
  disabledPlugins: string[];
  onTogglePlugin: (id: string) => void;
  onInstallPlugin: (source: string) => Promise<PluginManifest>;
  onUninstallPlugin: (id: string) => Promise<void>;
}

export function SettingsPanel({
  settings,
  onChange,
  onShortcutChange,
  shortcutConflict,
  activeShortcut,
  onLaunchAtStartupChange,
  onClearRecents,
  onResetSettings,
  onResetPosition,
  commands,
  onCommandsChange,
  manualApps,
  onAddManualApps,
  onRemoveManualApp,
  indexFolders,
  onAddIndexFolders,
  onRemoveIndexFolder,
  onRefreshIndexes,
  plugins,
  disabledPlugins,
  onTogglePlugin,
  onInstallPlugin,
  onUninstallPlugin,
}: SettingsPanelProps) {
  const { t } = useI18n();
  const [activeSection, setActiveSection] = useState<SectionId>(
    () => readRequestedSettingsSection() ?? DEFAULT_SECTION,
  );
  const [startupBusy, setStartupBusy] = useState(false);
  const { notice, showNotice } = useNotice();
  const { recording, toggleRecording, stopRecording } =
    useShortcutRecorder(onShortcutChange);
  const { exportConfiguration, importConfiguration } = useConfigurationTransfer({
    settings,
    commands,
    onChange,
    onCommandsChange,
    showNotice,
  });

  useEffect(() => {
    const applyRequestedSection = () => {
      const section = readRequestedSettingsSection();
      if (section) setActiveSection(section);
    };
    const onStorage = (event: StorageEvent) => {
      if (event.key !== SETTINGS_SECTION_REQUEST_KEY) return;
      applyRequestedSection();
    };
    window.addEventListener("storage", onStorage);
    window.addEventListener("focus", applyRequestedSection);
    return () => {
      window.removeEventListener("storage", onStorage);
      window.removeEventListener("focus", applyRequestedSection);
    };
  }, []);

  const toggleStartup = async () => {
    if (startupBusy) return;
    setStartupBusy(true);
    await onLaunchAtStartupChange(!settings.launchAtStartup);
    setStartupBusy(false);
  };

  const addManualApp = async () => {
    try {
      const path = await platform.pickExecutable();
      if (!path) return;
      onAddManualApps([{ path, name: appNameFromPath(path) }]);
      showNotice(t("toast.appAdded"));
    } catch {
      showNotice(t("toast.actionFailed"));
    }
  };

  const addIndexFolder = async () => {
    try {
      const folder = await platform.pickFolder();
      if (!folder) return;
      onAddIndexFolders([folder]);
      showNotice(t("toast.indexAdded"));
    } catch {
      showNotice(t("toast.actionFailed"));
    }
  };

  const refreshIndexes = () => {
    onRefreshIndexes();
    showNotice(t("toast.indexRefreshed"));
  };

  const activeMeta = SECTIONS.find((section) => section.id === activeSection)!;
  const pageSections = SECTIONS.filter(isSettingsSection);
  const upcomingSections = SECTIONS.filter(isLockedSection);

  return (
    <div className="settings-panel">
      <div className="settings-body">
        <nav className="settings-sidebar" aria-label={t("settings.title")}>
          {pageSections.map((section) => {
            const Icon = section.icon;
            return (
              <button
                key={section.id}
                type="button"
                className={`settings-nav-item${
                  activeSection === section.id ? " is-active" : ""
                }`}
                onClick={() => setActiveSection(section.id)}
              >
                <Icon size={14} strokeWidth={1.9} />
                <span>{t(section.labelKey)}</span>
                {section.badgeKey ? (
                  <span
                    className="settings-nav-badge"
                    title={
                      section.badgeTitleKey
                        ? t(section.badgeTitleKey)
                        : undefined
                    }
                  >
                    {t(section.badgeKey)}
                  </span>
                ) : null}
              </button>
            );
          })}

          {upcomingSections.length > 0 ? (
            <div className="settings-nav-caption">
              {t("settings.comingSoon")}
            </div>
          ) : null}

          {/* Teasers: greyed out, never selectable, no page behind them yet. */}
          {upcomingSections.map((section) => {
            const Icon = section.icon;
            return (
              <div
                key={section.id}
                className="settings-nav-item is-locked"
                aria-disabled="true"
                title={t(section.teaserKey)}
              >
                <Icon size={14} strokeWidth={1.9} />
                <span>{t(section.labelKey)}</span>
              </div>
            );
          })}
        </nav>

        <div className="settings-content">
          <div className="settings-pane-title">{t(activeMeta.labelKey)}</div>

          {activeSection === "profile" ? (
            <div className="profile-page">
              <div className="profile-hero">
                <span className="profile-avatar-large" aria-hidden="true">
                  <UserRound size={25} strokeWidth={1.7} />
                </span>
                <div className="profile-name-large">
                  {t("settings.profileLocal")}
                </div>
                <div className="profile-subline">{t("settings.profileHint")}</div>
                <div className="profile-device">
                  {t("settings.profileDataHint")}
                </div>
              </div>

              <section className="profile-section">
                <div className="profile-section-title">{t("settings.account")}</div>
                <div className="profile-list">
                  <div className="profile-list-row">
                    <div className="profile-row-copy">
                      <div className="profile-row-title">{t("settings.accountType")}</div>
                      <div className="profile-row-hint">{t("settings.accountTypeHint")}</div>
                    </div>
                    <span className="profile-row-value">{t("settings.accountLocal")}</span>
                  </div>
                  <div className="profile-list-row">
                    <div className="profile-row-copy">
                      <div className="profile-row-title">{t("settings.cloudSync")}</div>
                      <div className="profile-row-hint">{t("settings.cloudSyncHint")}</div>
                    </div>
                    <span className="profile-row-value">{t("settings.comingLater")}</span>
                  </div>
                </div>
              </section>

              <section className="profile-section">
                <div className="profile-section-title">{t("settings.backup")}</div>
                <div className="profile-list">
                  <div className="profile-list-row">
                    <div className="profile-row-copy">
                      <div className="profile-row-title">{t("settings.export")}</div>
                      <div className="profile-row-hint">
                        {t("settings.exportHint")}
                      </div>
                    </div>
                    <button
                      type="button"
                      className="profile-action"
                      onClick={() => void exportConfiguration()}
                    >
                      <Download size={13} strokeWidth={2} />
                      {t("settings.exportAction")}
                    </button>
                  </div>
                  <div className="profile-list-row">
                    <div className="profile-row-copy">
                      <div className="profile-row-title">{t("settings.import")}</div>
                      <div className="profile-row-hint">
                        {t("settings.importHint")}
                      </div>
                    </div>
                    <button
                      type="button"
                      className="profile-action"
                      onClick={() => void importConfiguration()}
                    >
                      <Upload size={13} strokeWidth={2} />
                      {t("settings.importAction")}
                    </button>
                  </div>
                </div>
              </section>

              <section className="profile-section">
                <div className="profile-section-title">{t("settings.localData")}</div>
                <div className="profile-list">
                  <div className="profile-list-row">
                    <div className="profile-row-copy">
                      <div className="profile-row-title">{t("settings.clearRecents")}</div>
                      <div className="profile-row-hint">{t("settings.clearRecentsHint")}</div>
                    </div>
                    <button
                      type="button"
                      className="profile-action is-danger"
                      onClick={onClearRecents}
                    >
                      <Trash2 size={13} strokeWidth={2} />
                      {t("settings.clearRecents")}
                    </button>
                  </div>
                  <div className="profile-list-row">
                    <div className="profile-row-copy">
                      <div className="profile-row-title">{t("settings.resetSettings")}</div>
                      <div className="profile-row-hint">{t("settings.resetSettingsHint")}</div>
                    </div>
                    <button
                      type="button"
                      className="profile-action is-danger"
                      onClick={onResetSettings}
                    >
                      <RotateCcw size={13} strokeWidth={2} />
                      {t("settings.resetSettings")}
                    </button>
                  </div>
                </div>
              </section>
            </div>
          ) : null}

          {activeSection === "appearance" ? (
            <>
              <div className="settings-row">
                <div className="settings-copy">
                  <div className="settings-label">{t("settings.theme")}</div>
                  <div className="settings-description">
                    {t("settings.themeHint")}
                  </div>
                </div>
                <Segmented
                  withIcons
                  label={t("settings.theme")}
                  value={settings.theme}
                  options={THEME_OPTIONS.map((option) => {
                    const Icon = option.icon;
                    return {
                      value: option.value,
                      label: t(option.labelKey),
                      icon: <Icon size={11} strokeWidth={2.1} />,
                    };
                  })}
                  onChange={(theme) => onChange({ theme })}
                />
              </div>

              <div className="settings-group">
                <div className="settings-group-title">
                  {t("settings.customize")}
                </div>

                <div className="settings-row">
                  <div className="settings-copy">
                    <div className="settings-label">{t("settings.textSize")}</div>
                    <div className="settings-description">
                      {t("settings.textSizeHint")}
                    </div>
                  </div>
                  <SliderRow
                    label={t("settings.textSize")}
                    min={FONT_SIZE_MIN}
                    max={FONT_SIZE_MAX}
                    value={settings.fontSize}
                    valueLabel={`${settings.fontSize}px`}
                    onChange={(fontSize) => onChange({ fontSize })}
                  />
                </div>

                <div className="settings-row">
                  <div className="settings-copy">
                    <div className="settings-label">{t("settings.radius")}</div>
                    <div className="settings-description">
                      {t("settings.radiusHint")}
                    </div>
                  </div>
                  <SliderRow
                    label={t("settings.radius")}
                    min={RADIUS_MIN}
                    max={RADIUS_MAX}
                    value={settings.radius}
                    valueLabel={`${settings.radius}px`}
                    onChange={(radius) => onChange({ radius })}
                  />
                </div>

                <div className="settings-row">
                  <div className="settings-copy">
                    <div className="settings-label">{t("settings.shadow")}</div>
                    <div className="settings-description">
                      {t("settings.shadowHint")}
                    </div>
                  </div>
                  <SliderRow
                    label={t("settings.shadow")}
                    min={SHADOW_MIN}
                    max={SHADOW_MAX}
                    value={settings.shadow}
                    valueLabel={`${settings.shadow}%`}
                    onChange={(shadow) => onChange({ shadow })}
                  />
                </div>

                <div className="appearance-preview" aria-hidden="true">
                  <div
                    className="appearance-preview-card"
                    style={{ borderRadius: `${settings.radius}px` }}
                  >
                    <div className="appearance-preview-search" />
                    <div className="appearance-preview-row" />
                    <div className="appearance-preview-row is-short" />
                  </div>
                  <span className="appearance-preview-caption">
                    {t("settings.radiusPreview")}
                  </span>
                </div>
              </div>

              <div className="settings-group">
                <div className="settings-group-title">
                  {t("settings.window")}
                </div>

                <div className="settings-block">
                  <div className="settings-block-title">
                    {t("settings.windowMode")}
                  </div>
                  <div className="settings-description">
                    {t("settings.windowModeHint")}
                  </div>
                  <div className="mode-grid">
                    <button
                      type="button"
                      className={`mode-card${settings.windowMode === "expanded" ? " is-active" : ""}`}
                      onClick={() => onChange({ windowMode: "expanded" })}
                    >
                      <span className="mode-preview" aria-hidden="true">
                        <span className="mode-search-bar" />
                        <span className="mode-row" />
                        <span className="mode-row mode-row-short" />
                      </span>
                      <span className="mode-copy">
                        <span className="mode-title">{t("settings.modeExpanded")}</span>
                        <span className="mode-hint">{t("settings.modeExpandedHint")}</span>
                      </span>
                    </button>
                    <button
                      type="button"
                      className={`mode-card${settings.windowMode === "compact" ? " is-active" : ""}`}
                      onClick={() => onChange({ windowMode: "compact" })}
                    >
                      <span className="mode-preview mode-preview-compact" aria-hidden="true">
                        <span className="mode-search-bar" />
                      </span>
                      <span className="mode-copy">
                        <span className="mode-title">{t("settings.modeCompact")}</span>
                        <span className="mode-hint">{t("settings.modeCompactHint")}</span>
                      </span>
                    </button>
                  </div>
                </div>

                <div className="settings-row">
                  <div className="settings-copy">
                    <div className="settings-label">
                      {t("settings.launcherAnimation")}
                    </div>
                    <div className="settings-description">
                      {t("settings.launcherAnimationHint")}
                    </div>
                  </div>
                  <Toggle
                    checked={settings.launcherAnimation}
                    onClick={() =>
                      onChange({
                        launcherAnimation: !settings.launcherAnimation,
                      })
                    }
                    label={t("settings.launcherAnimation")}
                  />
                </div>

                <div className="settings-row">
                  <div className="settings-copy">
                    <div className="settings-label">{t("settings.resetPosition")}</div>
                    <div className="settings-description">
                      {t("settings.resetPositionHint")}
                    </div>
                  </div>
                  <button type="button" className="settings-action" onClick={onResetPosition}>
                    <RotateCcw size={13} strokeWidth={2} />
                    {t("settings.resetPosition")}
                  </button>
                </div>
              </div>
            </>
          ) : null}

          {activeSection === "apps" ? (
            <AppsSettings
              manualApps={manualApps}
              indexFolders={indexFolders}
              onAddApp={() => void addManualApp()}
              onRemoveApp={onRemoveManualApp}
              onAddIndexFolder={() => void addIndexFolder()}
              onRemoveIndexFolder={onRemoveIndexFolder}
              onRefreshIndexes={refreshIndexes}
            />
          ) : null}

          {activeSection === "commands" ? (
            <CommandsSettings
              commands={commands}
              onChange={onCommandsChange}
            />
          ) : null}

          {activeSection === "features" ? (
            <FeaturesSettings settings={settings} onChange={onChange} />
          ) : null}

          {activeSection === "plugins" ? (
            <PluginsSettings
              plugins={plugins}
              disabled={disabledPlugins}
              onToggle={onTogglePlugin}
              onInstall={onInstallPlugin}
              onUninstall={onUninstallPlugin}
            />
          ) : null}

          {activeSection === "general" ? (
            <>
              <div className="settings-row">
                <div className="settings-copy">
                  <div className="settings-label">{t("settings.language")}</div>
                  <div className="settings-description">
                    {t("settings.languageHint")}
                  </div>
                </div>
                <Segmented
                  label={t("settings.language")}
                  value={settings.language}
                  options={[
                    { value: "en", label: t("settings.languageEnglish") },
                    { value: "zh", label: t("settings.languageChinese") },
                  ]}
                  onChange={(language) => onChange({ language })}
                />
              </div>

              <div className="settings-row">
                <div className="settings-copy">
                  <div className="settings-label">{t("settings.font")}</div>
                  <div className="settings-description">
                    {t("settings.fontHint")}
                  </div>
                </div>
                <select
                  className="font-select"
                  value={settings.fontFamily}
                  onChange={(event) =>
                    onChange({
                      fontFamily: event.target.value as Settings["fontFamily"],
                    })
                  }
                  aria-label={t("settings.font")}
                >
                  {FONT_OPTIONS.map((option) => (
                    <option key={option.value} value={option.value}>
                      {t(option.labelKey)}
                    </option>
                  ))}
                </select>
              </div>

              <div
                className="font-preview"
                style={{ fontFamily: FONT_STACKS[settings.fontFamily] }}
              >
                <span className="font-preview-label">
                  {t("settings.fontPreview")}
                </span>
                <span className="font-preview-text">
                  Aa · The quick brown fox · 搜索应用
                </span>
              </div>

              <div className={`settings-row${shortcutConflict ? " has-error" : ""}`}>
                <div className="settings-copy">
                  <div className="settings-label">{t("settings.shortcut")}</div>
                  <div className="settings-description">
                    {t("settings.shortcutHint")}
                  </div>
                </div>
                <div className="settings-shortcut">
                  <ShortcutKeys shortcut={settings.shortcut} />
                  <button
                    type="button"
                    className={`settings-action${recording ? " is-recording" : ""}`}
                    onClick={toggleRecording}
                  >
                    <Keyboard size={13} strokeWidth={2} />
                    {recording ? t("settings.recording") : t("settings.record")}
                  </button>
                </div>
              </div>

              {shortcutConflict ? (
                <div className="settings-warning" role="alert">
                  <AlertCircle size={13} strokeWidth={2} />
                  <span>
                    {t("settings.shortcutConflict")}
                    {activeShortcut && activeShortcut !== settings.shortcut
                      ? ` ${t("settings.shortcutFallback", {
                          shortcut: activeShortcut,
                        })}`
                      : ""}
                  </span>
                </div>
              ) : null}

              {recording ? (
                <div className="shortcut-presets">
                  <span className="shortcut-presets-label">
                    {t("settings.spaceShortcuts")}
                  </span>
                  {SHORTCUT_PRESETS.map((shortcut) => (
                    <button
                      key={shortcut}
                      type="button"
                      className="shortcut-preset"
                      onClick={() => {
                        void onShortcutChange(shortcut).then((accepted) => {
                          if (accepted) stopRecording();
                        });
                      }}
                    >
                      {shortcut.split("+").join(" + ")}
                    </button>
                  ))}
                </div>
              ) : null}

              <div className="settings-row">
                <div className="settings-copy">
                  <div className="settings-label">{t("settings.startup")}</div>
                  <div className="settings-description">
                    {t("settings.startupHint")}
                  </div>
                </div>
                <Toggle
                  checked={settings.launchAtStartup}
                  disabled={startupBusy}
                  onClick={() => void toggleStartup()}
                  label={t("settings.startup")}
                />
              </div>

              <div className="settings-group">
                <div className="settings-group-title">{t("settings.search")}</div>

                <div className="settings-row">
                  <div className="settings-copy">
                    <div className="settings-label">{t("settings.showRecent")}</div>
                    <div className="settings-description">
                      {t("settings.showRecentHint")}
                    </div>
                  </div>
                  <Toggle
                    checked={settings.showRecent}
                    onClick={() => onChange({ showRecent: !settings.showRecent })}
                    label={t("settings.showRecent")}
                  />
                </div>

                <div className={`settings-row${settings.showRecent ? "" : " is-disabled"}`}>
                  <div className="settings-copy">
                    <div className="settings-label">{t("settings.maxRecent")}</div>
                    <div className="settings-description">
                      {t("settings.maxRecentHint")}
                    </div>
                  </div>
                  <div className="segmented" role="group" aria-label={t("settings.maxRecent")}>
                    {[3, 5, 8].map((count) => (
                      <button
                        key={count}
                        type="button"
                        disabled={!settings.showRecent}
                        className={`segment${settings.maxRecent === count ? " is-active" : ""}`}
                        onClick={() => onChange({ maxRecent: count })}
                      >
                        {count}
                      </button>
                    ))}
                  </div>
                </div>

                <div className="settings-row">
                  <div className="settings-copy">
                    <div className="settings-label">{t("settings.fileSearch")}</div>
                    <div className="settings-description">
                      {t("settings.fileSearchHint")}
                    </div>
                  </div>
                  <Toggle
                    checked={settings.fileSearch}
                    onClick={() => onChange({ fileSearch: !settings.fileSearch })}
                    label={t("settings.fileSearch")}
                  />
                </div>
              </div>
            </>
          ) : null}
        </div>
      </div>

      {notice ? (
        <div className="toast" role="status">
          {notice}
        </div>
      ) : null}
    </div>
  );
}
