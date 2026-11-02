import { Command, LoaderCircle, Settings2 } from "lucide-react";

import { useI18n } from "../i18n";

interface FooterProps {
  indexing: boolean;
  fileCount: number | null;
  onSettings: () => void;
  onActions: () => void;
  showActions: boolean;
}

function formatCount(count: number): string {
  if (count >= 1000) {
    return `${(count / 1000).toFixed(count >= 10_000 ? 0 : 1)}k`;
  }
  return String(count);
}

export function Footer({
  indexing,
  fileCount,
  onSettings,
  onActions,
  showActions,
}: FooterProps) {
  const { t } = useI18n();

  return (
    <footer className="launcher-footer">
      <div className="footer-hints">
        {showActions ? (
          <button
            type="button"
            className="footer-action"
            onClick={onActions}
          >
            <Command size={11} strokeWidth={1.9} />
            <span>{t("footer.actions")}</span>
            <kbd className="key-text">Ctrl K</kbd>
          </button>
        ) : null}
      </div>

      <div className="footer-right">
        {indexing ? (
          <span className="index-status">
            <LoaderCircle className="spin" size={11} strokeWidth={2} />
            <span>{t("footer.indexing")}</span>
          </span>
        ) : fileCount !== null ? (
          <span className="index-status">
            {t("footer.files", { count: formatCount(fileCount) })}
          </span>
        ) : null}

        <button
          type="button"
          className="settings-button"
          onClick={onSettings}
          aria-label={t("settings.title")}
          title={t("settings.title")}
        >
          <Settings2 size={14} strokeWidth={1.9} />
        </button>
      </div>
    </footer>
  );
}
