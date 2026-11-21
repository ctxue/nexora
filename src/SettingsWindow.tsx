import { useCallback, useEffect, useState } from "react";

import { SettingsPanel } from "./components/SettingsPanel";
import { useManualApps } from "./features/apps/useManualApps";
import { useCommands } from "./features/commands/useCommands";
import { useIndexFolders } from "./features/indexes/useIndexFolders";
import { usePlugins } from "./features/plugins/usePlugins";
import { DEFAULT_SETTINGS } from "./features/settings/defaults";
import { useSettings } from "./features/settings/useSettings";
import { I18nProvider, useI18n } from "./i18n";
import { platform } from "./platform/tauri";

const RECENTS_KEY = "nexora.recents.v1";

export default function SettingsWindow() {
  const { settings, updateSettings } = useSettings();

  return (
    <I18nProvider language={settings.language}>
      <SettingsPage settings={settings} updateSettings={updateSettings} />
    </I18nProvider>
  );
}

function SettingsPage({
  settings,
  updateSettings,
}: {
  settings: ReturnType<typeof useSettings>["settings"];
  updateSettings: ReturnType<typeof useSettings>["updateSettings"];
}) {
  const { t } = useI18n();
  const { commands, setCommands } = useCommands();
  const { apps: manualApps, addApps, removeApp } = useManualApps();
  const {
    folders: indexFolders,
    addFolders,
    removeFolder,
    refresh: refreshIndexes,
  } = useIndexFolders();
  const {
    plugins,
    disabled: disabledPlugins,
    toggle: togglePlugin,
    install: installPlugin,
    uninstall: uninstallPlugin,
  } = usePlugins();
  const [shortcutConflict, setShortcutConflict] = useState(false);
  const [activeShortcut, setActiveShortcut] = useState<string | null>(null);

  const refreshShortcutStatus = useCallback(async () => {
    try {
      const status = await platform.getShortcutStatus();
      setShortcutConflict(Boolean(status.error));
      setActiveShortcut(status.current);
    } catch {
    }
  }, []);

  const applyShortcut = useCallback(
    async (shortcut: string) => {
      try {
        await platform.setGlobalShortcut(shortcut);
        setShortcutConflict(false);
        setActiveShortcut(shortcut);
        updateSettings({ shortcut });
        return true;
      } catch {
        updateSettings({ shortcut });
        await refreshShortcutStatus();
        return false;
      }
    },
    [refreshShortcutStatus, updateSettings],
  );

  const applyLaunchAtStartup = useCallback(
    async (enabled: boolean) => {
      try {
        await platform.setLaunchAtStartup(enabled);
        updateSettings({ launchAtStartup: enabled });
        return true;
      } catch {
        return false;
      }
    },
    [updateSettings],
  );

  const resetAllSettings = useCallback(async () => {
    try {
      await platform.setGlobalShortcut(DEFAULT_SETTINGS.shortcut);
      await platform.setLaunchAtStartup(false);
    } catch {
    }
    updateSettings(DEFAULT_SETTINGS);
    await refreshShortcutStatus();
  }, [refreshShortcutStatus, updateSettings]);

  useEffect(() => {
    void refreshShortcutStatus();
    const onFocus = () => void refreshShortcutStatus();
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [refreshShortcutStatus]);

  useEffect(() => {
    void platform
      .getLaunchAtStartup()
      .then((enabled) => {
        if (enabled !== settings.launchAtStartup) {
          updateSettings({ launchAtStartup: enabled });
        }
      })
      .catch(() => {});
  }, []);

  useEffect(() => {
    void platform.setTrayLanguage(settings.language).catch(() => {});
  }, [settings.language]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        void platform.closeCurrentWindow();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  return (
    <main className="settings-window" aria-label={t("settings.title")}>
      <SettingsPanel
        settings={settings}
        onChange={updateSettings}
        onShortcutChange={applyShortcut}
        shortcutConflict={shortcutConflict}
        activeShortcut={activeShortcut}
        onLaunchAtStartupChange={applyLaunchAtStartup}
        onClearRecents={() => {
          window.localStorage.removeItem(RECENTS_KEY);
        }}
        onResetSettings={() => void resetAllSettings()}
        onResetPosition={() => void platform.resetWindowPosition()}
        commands={commands}
        onCommandsChange={setCommands}
        manualApps={manualApps}
        onAddManualApps={addApps}
        onRemoveManualApp={removeApp}
        indexFolders={indexFolders}
        onAddIndexFolders={addFolders}
        onRemoveIndexFolder={removeFolder}
        onRefreshIndexes={refreshIndexes}
        plugins={plugins}
        disabledPlugins={disabledPlugins}
        onTogglePlugin={togglePlugin}
        onInstallPlugin={installPlugin}
        onUninstallPlugin={uninstallPlugin}
      />
    </main>
  );
}
