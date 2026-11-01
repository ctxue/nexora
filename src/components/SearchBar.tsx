import { X } from "lucide-react";
import type { RefObject } from "react";

import type { CommandIcon } from "../core/types";
import { useI18n } from "../i18n";
import { BrandMark } from "./BrandMark";
import { CommandGlyphIcon } from "./CommandGlyph";
import { SearchCaret } from "./SearchCaret";

interface SearchBarProps {
  value: string;
  shortcut: string;
  inputRef: RefObject<HTMLInputElement | null>;
  onChange: (value: string) => void;
  onClear: () => void;
  onDragStart: () => void;
  /** Active command scope: the typed keyword became a chip. */
  scope?: { title: string; keyword: string; icon: CommandIcon } | null;
  onClearScope?: () => void;
}

export function SearchBar({
  value,
  shortcut,
  inputRef,
  onChange,
  onClear,
  onDragStart,
  scope = null,
  onClearScope,
}: SearchBarProps) {
  const { t } = useI18n();

  return (
    <div
      className="search-bar"
      data-tauri-drag-region
      onMouseDown={(event) => {
        if (event.button !== 0) return;
        const target = event.target as HTMLElement;
        if (target.closest("input, button")) return;
        event.preventDefault();
        onDragStart();
      }}
    >
      <span
        className="brand-mark"
        data-tauri-drag-region
        aria-hidden="true"
        onPointerDown={(event) => {
          event.stopPropagation();
          if (event.button !== 0) return;
          event.preventDefault();
          onDragStart();
        }}
        onMouseDown={(event) => event.stopPropagation()}
      >
        <BrandMark />
      </span>

      {scope ? (
        <span className="search-scope" title={t("search.clearScope")}>
          <CommandGlyphIcon icon={scope.icon} size={13} strokeWidth={1.9} />
          <span className="search-scope-label">{scope.title}</span>
        </span>
      ) : null}

      <input
        ref={inputRef}
        className="search-input"
        value={value}
        onChange={(event) => onChange(event.target.value)}
        onKeyDown={(event) => {
          if (scope && event.key === "Backspace" && value === "") {
            event.preventDefault();
            onClearScope?.();
          }
        }}
        placeholder={
          scope
            ? t("search.scopedPlaceholder", { name: scope.title })
            : t("search.placeholder")
        }
        aria-label={
          scope
            ? t("search.scopedPlaceholder", { name: scope.title })
            : t("search.placeholder")
        }
        autoComplete="off"
        autoCorrect="off"
        spellCheck={false}
        autoFocus
      />

      <SearchCaret inputRef={inputRef} />

      {value ? (
        <button
          type="button"
          className="clear-button"
          onClick={onClear}
          aria-label={t("search.clear")}
        >
          <X size={14} strokeWidth={2} />
        </button>
      ) : null}

      <span className="shortcut-hint" aria-label={shortcut}>
        {shortcut.split("+").map((part, index) => (
          <kbd key={`${part}-${index}`}>{part}</kbd>
        ))}
      </span>
    </div>
  );
}
