import { Clipboard, Fingerprint, LockKeyhole, ShieldCheck } from "lucide-react";

import type { Settings } from "../../features/settings/types";
import { useI18n } from "../../i18n";
import { Toggle } from "./Controls";

export function ClipboardSettings({
  settings,
  onChange,
}: {
  settings: Settings;
  onChange: (patch: Partial<Settings>) => void;
}) {
  const { t } = useI18n();
  const enabled = settings.clipboardHistory;

  return (
    <div className="feature-page">
      <div className="feature-hero">
        <span className="feature-icon" aria-hidden="true">
          <Clipboard size={20} strokeWidth={1.8} />
        </span>
        <div className="feature-hero-copy">
          <div className="feature-hero-title-line">
            <span className="feature-hero-title">
              {t("settings.clipboardHistory")}
            </span>
            <span className={`feature-status${enabled ? " is-on" : ""}`}>
              {enabled
                ? t("settings.clipboardStatusOn")
                : t("settings.clipboardStatusOff")}
            </span>
          </div>
          <p className="feature-hero-description">
            {t("settings.clipboardHistoryHint")}
          </p>
        </div>
        <Toggle
          checked={enabled}
          onClick={() => onChange({ clipboardHistory: !enabled })}
          label={t("settings.clipboardHistory")}
        />
      </div>

      <div className="feature-list">
        <div className="feature-list-row">
          <LockKeyhole size={15} strokeWidth={1.8} />
          <div>
            <div className="feature-list-title">
              {t("settings.clipboardEncrypted")}
            </div>
            <div className="feature-list-description">
              {t("settings.clipboardEncryptedHint")}
            </div>
          </div>
        </div>
        <div className="feature-list-row">
          <ShieldCheck size={15} strokeWidth={1.8} />
          <div>
            <div className="feature-list-title">
              {t("settings.clipboardSensitive")}
            </div>
            <div className="feature-list-description">
              {t("settings.clipboardSensitiveHint")}
            </div>
          </div>
        </div>
        <div className="feature-list-row">
          <Fingerprint size={15} strokeWidth={1.8} />
          <div>
            <div className="feature-list-title">
              {t("settings.clipboardWindows")}
            </div>
            <div className="feature-list-description">
              {t("settings.clipboardWindowsHint")}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
