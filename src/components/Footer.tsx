import {
  ArrowDown,
  ArrowUp,
  CornerDownLeft,
  LoaderCircle,
  Settings2,
} from "lucide-react";

import { useI18n } from "../i18n";

interface FooterProps {
  indexing: boolean;
  fileCount: number | null;
  onSettings: () => void;
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
  showActions,
}: FooterProps) {
  const { t } = useI18n();

  return (
    <footer className="launcher-footer">
      <div className="footer-hints">
        <span className="footer-hint">
          <kbd className="key-icon">
            <ArrowUp size={10} strokeWidth={2.2} />
          </kbd>
          <kbd className="key-icon">
            <ArrowDown size={10} strokeWidth={2.2} />
          </kbd>
          <span>{t("footer.navigate")}</span>
        </span>

        <span className="footer-hint">
          <kbd className="key-icon">
            <CornerDownLeft size={10} strokeWidth={2.2} />
          </kbd>
          <span>{t("footer.open")}</span>
        </span>

        {showActions ? (
          <span className="footer-hint">
            <kbd className="key-text">Ctrl</kbd>
            <kbd className="key-icon">
              <CornerDownLeft size={10} strokeWidth={2.2} />
            </kbd>
            <span>{t("footer.actions")}</span>
          </span>
        ) : null}

        <span className="footer-hint">
          <kbd className="key-text">esc</kbd>
          <span>{t("footer.close")}</span>
        </span>
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
