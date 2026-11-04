import { CornerDownLeft, SearchX } from "lucide-react";
import { useEffect, useRef, type CSSProperties } from "react";

import type { LauncherItem, ResultGroup } from "../core/types";
import { useI18n } from "../i18n";
import { ResultIcon } from "./ResultIcon";

interface ResultListProps {
  groups: ResultGroup[];
  query: string;
  icons: Record<string, string>;
  selectedIndex: number;
  onSelect: (index: number) => void;
  onExecute: (item: LauncherItem) => void;
  onActions: (item: LauncherItem) => void;
}

export function ResultList({
  groups,
  query,
  icons,
  selectedIndex,
  onSelect,
  onExecute,
  onActions,
}: ResultListProps) {
  const { t } = useI18n();
  const selectedRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    selectedRef.current?.scrollIntoView({ block: "nearest" });
  }, [selectedIndex]);

  if (groups.length === 0) {
    return (
      <div className="no-results">
        <span className="no-results-icon" aria-hidden="true">
          <SearchX size={19} strokeWidth={1.7} />
        </span>
        <div className="no-results-title">
          {t("results.noResults", { query: query.trim() })}
        </div>
        <div className="no-results-copy">{t("results.try")}</div>
      </div>
    );
  }

  let itemIndex = -1;

  return (
    <div className="results-scroll" role="listbox" aria-label="Search results">
      {groups.map((group) => (
        <section
          className="result-group"
          key={group.id}
          style={{ "--group-color": group.color } as CSSProperties}
          aria-label={group.label}
        >
          <div className="group-header">
            <span className="group-dot" aria-hidden="true" />
            <span className="group-label">{group.label}</span>
            <span className="group-count">{group.items.length}</span>
          </div>

          {group.items.map((item) => {
            itemIndex += 1;
            const index = itemIndex;
            const selected = index === selectedIndex;

            return (
              <button
                key={item.id}
                ref={selected ? selectedRef : undefined}
                type="button"
                role="option"
                aria-selected={selected}
                className={`result-row${selected ? " is-selected" : ""}`}
                style={{ "--row-index": index } as CSSProperties}
                onMouseEnter={() => onSelect(index)}
                onFocus={() => onSelect(index)}
                onClick={() => onExecute(item)}
                onContextMenu={(event) => {
                  event.preventDefault();
                  onSelect(index);
                  onActions(item);
                }}
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

                <span className="result-tail">
                  {item.kind === "file" && item.extension ? (
                    <span className="extension-badge">
                      {item.extension.toUpperCase().slice(0, 4)}
                    </span>
                  ) : null}
                  <span
                    className={`enter-hint${selected ? " is-visible" : ""}`}
                    aria-hidden="true"
                  >
                    <CornerDownLeft size={12} strokeWidth={2} />
                  </span>
                </span>
              </button>
            );
          })}
        </section>
      ))}
    </div>
  );
}
