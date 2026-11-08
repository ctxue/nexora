import { useCallback } from "react";

import type { CustomCommand } from "../../core/commands";
import { exportPayload, settingsFromExport } from "../../features/settings/transfer";
import type { Settings } from "../../features/settings/types";
import { useI18n } from "../../i18n";
import { platform } from "../../platform/tauri";

export function useConfigurationTransfer({
  settings,
  commands,
  onChange,
  onCommandsChange,
  showNotice,
}: {
  settings: Settings;
  commands: CustomCommand[];
  onChange: (patch: Partial<Settings>) => void;
  onCommandsChange: (commands: CustomCommand[]) => void;
  showNotice: (message: string) => void;
}) {
  const { t } = useI18n();

  const exportConfiguration = useCallback(async () => {
    try {
      const json = JSON.stringify(exportPayload(settings, commands), null, 2);
      const path = await platform.exportSettings(json);
      if (path) showNotice(t("toast.exported", { path }));
    } catch {
      showNotice(t("toast.exportFailed"));
    }
  }, [commands, settings, showNotice, t]);

  const importConfiguration = useCallback(async () => {
    let contents: string | null;
    try {
      contents = await platform.importSettings();
    } catch {
      showNotice(t("toast.importFailed"));
      return;
    }
    if (!contents) return;

    const loaded = settingsFromExport(contents);
    if (!loaded) {
      showNotice(t("toast.importInvalid"));
      return;
    }

    onChange(loaded.settings);
    if (loaded.hasCommands) onCommandsChange(loaded.commands);

    await Promise.allSettled([
      platform.setGlobalShortcut(loaded.settings.shortcut),
      platform.setLaunchAtStartup(loaded.settings.launchAtStartup),
      platform.setTrayLanguage(loaded.settings.language),
    ]);
    showNotice(t("toast.imported"));
  }, [onChange, onCommandsChange, showNotice, t]);

  return { exportConfiguration, importConfiguration };
}
