import {
  Clipboard,
  CornerDownLeft,
  Pin,
  PinOff,
  Trash2,
} from "lucide-react";
import { useEffect, useRef } from "react";

import { buildClipboardPreview } from "../features/clipboard/filter";
import type { ClipboardEntry } from "../platform/tauri";
import { useI18n } from "../i18n";

interface ClipboardListProps {
  entries: ClipboardEntry[];
  selectedIndex: number;
  onSelect: (index: number) => void;
  onCopy: (entry: ClipboardEntry) => void;
  onTogglePin: (entry: ClipboardEntry) => void;
  onRemove: (entry: ClipboardEntry) => void;
  onClear: () => void;
}

function formatTimestamp(value: number): string {
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(value));
}

export function ClipboardList({
  entries,
  selectedIndex,
  onSelect,
  onCopy,
  onTogglePin,
  onRemove,
  onClear,
}: ClipboardListProps) {
  const { t } = useI18n();
  const selectedRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    selectedRef.current?.scrollIntoView({ block: "nearest" });
  }, [selectedIndex]);

  return (
    <div className="clipboard-panel">
      <div className="clipboard-header">
        <span className="clipboard-heading">
          <Clipboard size={12} strokeWidth={2} />
          {t("clipboard.history")}
          <span className="clipboard-count">{entries.length}</span>
        </span>
        <button
          type="button"
          className="clipboard-clear"
          onClick={onClear}
          disabled={entries.length === 0}
        >
          {t("clipboard.clear")}
        </button>
      </div>

      {entries.length === 0 ? (
        <div className="clipboard-empty">
          <Clipboard size={20} strokeWidth={1.6} />
          <span>{t("clipboard.empty")}</span>
        </div>
      ) : (
        <div
          className="clipboard-list"
          role="listbox"
          aria-label={t("clipboard.history")}
        >
          {entries.map((entry, index) => {
            const selected = index === selectedIndex;
            return (
              <div
                className={`clipboard-row${selected ? " is-selected" : ""}`}
                key={entry.id}
                onMouseEnter={() => onSelect(index)}
              >
                <button
                  ref={selected ? selectedRef : undefined}
                  type="button"
                  role="option"
                  aria-selected={selected}
                  className="clipboard-main"
                  onFocus={() => onSelect(index)}
                  onClick={() => onCopy(entry)}
                >
                  <span className="clipboard-preview">
                    {buildClipboardPreview(entry.text)}
                  </span>
                  <span className="clipboard-meta">
                    {entry.pinned ? t("clipboard.pinned") : formatTimestamp(entry.createdAt)}
                  </span>
                </button>

                <div className="clipboard-actions">
                  <button
                    type="button"
                    className={`clipboard-icon-action${entry.pinned ? " is-pinned" : ""}`}
                    onClick={() => onTogglePin(entry)}
                    aria-label={
                      entry.pinned ? t("clipboard.unpin") : t("clipboard.pin")
                    }
                    title={entry.pinned ? t("clipboard.unpin") : t("clipboard.pin")}
                  >
                    {entry.pinned ? (
                      <PinOff size={12} strokeWidth={2} />
                    ) : (
                      <Pin size={12} strokeWidth={2} />
                    )}
                  </button>
                  <button
                    type="button"
                    className="clipboard-icon-action is-danger"
                    onClick={() => onRemove(entry)}
                    aria-label={t("clipboard.remove")}
                    title={t("clipboard.remove")}
                  >
                    <Trash2 size={12} strokeWidth={2} />
                  </button>
                  <span
                    className={`clipboard-enter${selected ? " is-visible" : ""}`}
                    aria-hidden="true"
                  >
                    <CornerDownLeft size={12} strokeWidth={2} />
                  </span>
                </div>
              </div>
            );
          })}
        </div>
      )}

      <div className="clipboard-note">{t("clipboard.note")}</div>
    </div>
  );
}
