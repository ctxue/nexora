import { FolderOpen, Plus, Trash2 } from "lucide-react";
import { useState } from "react";

import { platform, type PluginManifest } from "../../platform/tauri";
import { useI18n } from "../../i18n";
import { CommandGlyph } from "../CommandGlyph";
import { ConfirmDialog } from "../ConfirmDialog";
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
  const [pending, setPending] = useState<
    | { kind: "install"; source: string }
    | { kind: "uninstall"; plugin: PluginManifest }
    | null
  >(null);

  const installFrom = async (source: string) => {
    setMessage(null);
    setBusy(true);
    try {
      const manifest = await onInstall(source);
      setMessage(t("plugins.installed", { name: manifest.name }));
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  };

  const install = async () => {
    setMessage(null);
    const folder = await platform.pickFolder();
    if (!folder) return;
    setPending({ kind: "install", source: folder });
  };

  const openDirectory = async () => {
    try {
      const directory = await platform.pluginsDir();
      await platform.openPath(directory);
    } catch {
      setMessage(t("plugins.openFailed"));
    }
  };

  const uninstallFrom = async (plugin: PluginManifest) => {
    setMessage(null);
    try {
      await onUninstall(plugin.id);
      setMessage(t("plugins.uninstalled", { name: plugin.name }));
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const confirmPending = async () => {
    if (!pending || busy) return;
    setBusy(true);
    try {
      if (pending.kind === "install") {
        await installFrom(pending.source);
      } else {
        await uninstallFrom(pending.plugin);
      }
    } finally {
      setBusy(false);
      setPending(null);
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
                  onClick={() => setPending({ kind: "uninstall", plugin })}
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

      {pending ? (
        <ConfirmDialog
          title={
            pending.kind === "install"
              ? t("plugins.installTitle")
              : t("plugins.confirmUninstall", {
                  name: pending.plugin.name,
                })
          }
          description={
            pending.kind === "install"
              ? t("plugins.confirmInstall")
              : t("plugins.confirmUninstallDescription")
          }
          cancelLabel={t("common.cancel")}
          confirmLabel={
            pending.kind === "install"
              ? t("plugins.install")
              : t("commands.delete")
          }
          tone={pending.kind === "uninstall" ? "danger" : "default"}
          busy={busy}
          onCancel={() => {
            if (!busy) setPending(null);
          }}
          onConfirm={() => void confirmPending()}
        />
      ) : null}
    </div>
  );
}
