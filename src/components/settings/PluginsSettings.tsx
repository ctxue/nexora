import { FolderOpen, Plus, Trash2 } from "lucide-react";
import { useState } from "react";

import { platform, type PluginManifest } from "../../platform/tauri";
import { useI18n } from "../../i18n";
import { CommandGlyph } from "../CommandGlyph";
import { Toggle } from "./Controls";

interface PluginsSettingsProps {
  plugins: PluginManifest[];
  disabled: string[];
  onToggle: (id: string) => void;
  onInstall: (source: string) => Promise<PluginManifest>;
  onUninstall: (id: string) => Promise<void>;
}

export function PluginsSettings({
  plugins,
  disabled,
  onToggle,
  onInstall,
  onUninstall,
}: PluginsSettingsProps) {
  const { t } = useI18n();
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  const install = async () => {
    setMessage(null);
    const folder = await platform.pickFolder();
    if (!folder) return;

    setBusy(true);
    try {
      const manifest = await onInstall(folder);
      setMessage(t("plugins.installed", { name: manifest.name }));
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  };

  const openDirectory = async () => {
    try {
      const directory = await platform.pluginsDir();
      await platform.openPath(directory);
    } catch {
      setMessage(t("plugins.openFailed"));
    }
  };

  const uninstall = async (plugin: PluginManifest) => {
    if (!window.confirm(t("plugins.confirmUninstall", { name: plugin.name }))) {
      return;
    }
    setMessage(null);
    try {
      await onUninstall(plugin.id);
      setMessage(t("plugins.uninstalled", { name: plugin.name }));
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  return (
    <div className="commands-page">
      <div className="commands-intro">
        <div className="settings-description">{t("settings.pluginsHint")}</div>
        <div className="plugins-actions">
          <button
            type="button"
            className="settings-action"
            onClick={() => void openDirectory()}
          >
            <FolderOpen size={13} strokeWidth={2} />
            {t("plugins.openFolder")}
          </button>
          <button
            type="button"
            className="settings-action"
            onClick={() => void install()}
            disabled={busy}
          >
            <Plus size={13} strokeWidth={2} />
            {t("plugins.install")}
          </button>
        </div>
      </div>

      {message ? (
        <div className="command-preview">
          <span className="command-preview-label">{t("plugins.status")}</span>
          <span className="command-preview-value">{message}</span>
        </div>
      ) : null}

      {plugins.length === 0 ? (
        <div className="commands-empty">{t("plugins.empty")}</div>
      ) : (
        <div className="commands-list">
          {plugins.map((plugin) => (
            <div
              key={plugin.id}
              className={`command-row${
                disabled.includes(plugin.id) ? " is-disabled" : ""
              }`}
            >
              <CommandGlyph icon="terminal" />

              <div className="command-copy">
                <div className="command-title-line">
                  <span className="command-title">{plugin.name}</span>
                  <span className="command-keyword">v{plugin.version}</span>
                  {plugin.author ? (
                    <span className="plugin-author">{plugin.author}</span>
                  ) : null}
                </div>
                <div className="command-subtitle">
                  {plugin.description || plugin.id}
                </div>
                <div className="plugin-meta">
                  {plugin.keywords.map((keyword) => (
                    <span className="command-keyword" key={keyword}>
                      {keyword}
                    </span>
                  ))}
                  {plugin.permissions.map((permission) => (
                    <span className="plugin-permission" key={permission}>
                      {permission}
                    </span>
                  ))}
                </div>
              </div>

              <div className="command-row-actions">
                <Toggle
                  checked={!disabled.includes(plugin.id)}
                  onClick={() => onToggle(plugin.id)}
                  label={plugin.name}
                />
                <button
                  type="button"
                  className="command-icon-button is-danger"
                  onClick={() => void uninstall(plugin)}
                  aria-label={t("commands.delete")}
                  title={t("commands.delete")}
                >
                  <Trash2 size={12} strokeWidth={2} />
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      <div className="plugin-hint">{t("plugins.hint")}</div>
    </div>
  );
}
