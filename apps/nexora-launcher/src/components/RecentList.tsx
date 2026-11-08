import { RotateCcw, X } from "lucide-react";
import type { CSSProperties } from "react";

import type { LauncherItem } from "../core/types";
import { useI18n } from "../i18n";
import { ResultIcon } from "./ResultIcon";

interface RecentListProps {
  items: LauncherItem[];
  icons: Record<string, string>;
  selectedIndex: number;
  onSelect: (index: number) => void;
  onOpen: (item: LauncherItem) => void;
  onRemove: (id: string) => void;
  onClear: () => void;
}

export function RecentList({
  items,
  icons,
  selectedIndex,
  onSelect,
  onOpen,
  onRemove,
  onClear,
}: RecentListProps) {
  const { t } = useI18n();

  return (
    <div className="start-panel">
      <div className="start-panel-header">
        <span className="start-panel-label">
          <RotateCcw size={11} strokeWidth={2} />
          {t("recent.label")}
        </span>
        <button
          type="button"
          className="start-panel-clear"
          onClick={onClear}
        >
          {t("recent.clear")}
        </button>
      </div>

      <div className="start-panel-list" role="listbox" aria-label="Recent items">
        {items.map((item, index) => {
          const selected = index === selectedIndex;

          return (
            <div
              className={`recent-row${selected ? " is-selected" : ""}`}
              key={item.id}
              style={{ "--row-index": index } as CSSProperties}
              onMouseEnter={() => onSelect(index)}
            >
              <button
                type="button"
                role="option"
                aria-selected={selected}
                className="recent-main"
                onFocus={() => onSelect(index)}
                onClick={() => onOpen(item)}
              >
                <span className="result-icon" aria-hidden="true">
                  <ResultIcon
                    item={item}
                    icon={item.path ? icons[item.path] : undefined}
                  />
                </span>

                <span className="result-copy">
                  <span className="result-title">{item.title}</span>
                  <span className="result-subtitle">{item.subtitle}</span>
                </span>
              </button>

              <button
                type="button"
                className="recent-remove"
                onClick={() => onRemove(item.id)}
                aria-label={`Remove ${item.title} from recent`}
                title="Remove"
              >
                <X size={12} strokeWidth={2} />
              </button>
            </div>
          );
        })}
      </div>

    </div>
  );
}
