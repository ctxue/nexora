import { FolderOpen, FolderPlus, Plus, RotateCcw, Trash2 } from "lucide-react";

import type { ManualApp } from "../../features/apps/storage";
import { useI18n } from "../../i18n";
import { CommandGlyphIcon } from "../CommandGlyph";

interface AppsSettingsProps {
  manualApps: ManualApp[];
  indexFolders: Array<{ id: string; path: string }>;
  onAddApp: () => void;
  onRemoveApp: (id: string) => void;
  onAddIndexFolder: () => void;
  onRemoveIndexFolder: (id: string) => void;
  onRefreshIndexes: () => void;
}

export function AppsSettings({
  manualApps,
  indexFolders,
  onAddApp,
  onRemoveApp,
  onAddIndexFolder,
  onRemoveIndexFolder,
  onRefreshIndexes,
}: AppsSettingsProps) {
  const { t } = useI18n();

  return (
    <div className="apps-page">
      <div className="settings-group">
        <div className="settings-group-title">{t("settings.addedApps")}</div>
        <div className="settings-description">
          {t("settings.appsHint")}
        </div>

        {manualApps.length > 0 ? (
          <div className="manual-apps">
            {manualApps.map((app) => (
              <div className="manual-app-row" key={app.id}>
                <span className="manual-app-icon" aria-hidden="true">
                  <CommandGlyphIcon icon="terminal" size={12} />
                </span>
                <div className="manual-app-copy">
                  <div className="manual-app-name">{app.name}</div>
                  <div className="manual-app-path">{app.path}</div>
                </div>
                <button
                  type="button"
                  className="command-icon-button is-danger"
                  onClick={() => onRemoveApp(app.id)}
                  aria-label={t("settings.removeApp")}
                  title={t("settings.removeApp")}
                >
                  <Trash2 size={12} strokeWidth={2} />
                </button>
              </div>
            ))}
          </div>
        ) : (
          <div className="manual-apps-empty">{t("settings.appsEmpty")}</div>
        )}

        <div className="manual-apps-actions">
          <button type="button" className="settings-action" onClick={onAddApp}>
            <Plus size={13} strokeWidth={2} />
            {t("settings.addApp")}
          </button>
        </div>
      </div>

      <div className="settings-group">
        <div className="settings-group-title">{t("settings.indexes")}</div>
        <div className="settings-description">
          {t("settings.indexesHint")}
        </div>

        {indexFolders.length > 0 ? (
          <div className="manual-apps">
            {indexFolders.map((folder) => (
              <div className="manual-app-row" key={folder.id}>
                <span className="manual-app-icon" aria-hidden="true">
                  <FolderOpen size={12} strokeWidth={2} />
                </span>
                <div className="manual-app-copy">
                  <div className="manual-app-name">{folder.path}</div>
                  <div className="manual-app-path">
                    {t("settings.indexFolderHint")}
                  </div>
                </div>
                <button
                  type="button"
                  className="command-icon-button is-danger"
                  onClick={() => onRemoveIndexFolder(folder.id)}
                  aria-label={t("settings.removeIndex")}
                  title={t("settings.removeIndex")}
                >
                  <Trash2 size={12} strokeWidth={2} />
                </button>
              </div>
            ))}
          </div>
        ) : (
          <div className="manual-apps-empty">
            {t("settings.indexesEmpty")}
          </div>
        )}

        <div className="manual-apps-actions">
          <button
            type="button"
            className="settings-action"
            onClick={onAddIndexFolder}
          >
            <FolderPlus size={13} strokeWidth={2} />
            {t("settings.addIndexFolder")}
          </button>
          <button
            type="button"
            className="settings-action"
            onClick={onRefreshIndexes}
            disabled={indexFolders.length === 0}
          >
            <RotateCcw size={13} strokeWidth={2} />
            {t("settings.rescanIndexes")}
          </button>
        </div>
      </div>
    </div>
  );
}
