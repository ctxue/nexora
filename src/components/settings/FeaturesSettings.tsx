import type { Settings } from "../../features/settings/types";
import { useI18n } from "../../i18n";
import { ClipboardSettings } from "./ClipboardSettings";

export function FeaturesSettings({
  settings,
  onChange,
}: {
  settings: Settings;
  onChange: (patch: Partial<Settings>) => void;
}) {
  const { t } = useI18n();

  return (
    <div className="features-page">
      <div className="features-intro">{t("settings.featuresHint")}</div>
      <ClipboardSettings settings={settings} onChange={onChange} />
    </div>
  );
}
