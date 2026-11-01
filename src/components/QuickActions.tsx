import { Sparkles } from "lucide-react";
import type { CSSProperties } from "react";

import type { LauncherItem } from "../core/types";
import { useI18n } from "../i18n";
import { ResultIcon } from "./ResultIcon";

interface QuickActionsProps {
  items: LauncherItem[];
  selectedIndex: number;
  onSelect: (index: number) => void;
  onOpen: (item: LauncherItem) => void;
}

export function QuickActions({
  items,
  selectedIndex,
  onSelect,
  onOpen,
}: QuickActionsProps) {
  const { t } = useI18n();

  return (
    <div className="start-panel">
      <div className="start-panel-header">
        <span className="start-panel-label">
          <Sparkles size={11} strokeWidth={2} />
          {t("quickActions.label")}
        </span>
      </div>

      <div className="start-panel-list" role="listbox" aria-label="Quick actions">
        {items.map((item, index) => {
          const selected = index === selectedIndex;

          return (
            <button
              type="button"
              role="option"
              aria-selected={selected}
              className={`start-action-row${selected ? " is-selected" : ""}`}
              key={item.id}
              style={{ "--row-index": index } as CSSProperties}
              onMouseEnter={() => onSelect(index)}
              onFocus={() => onSelect(index)}
              onClick={() => onOpen(item)}
            >
              <span className="result-icon" aria-hidden="true">
                <ResultIcon item={item} />
              </span>
              <span className="result-copy">
                <span className="result-title">{item.title}</span>
                <span className="result-subtitle">{item.subtitle}</span>
              </span>
            </button>
          );
        })}
      </div>

      <div className="start-panel-note">
        {t("quick.note")}
      </div>
    </div>
  );
}
