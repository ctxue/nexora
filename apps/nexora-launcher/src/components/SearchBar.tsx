import { X } from "lucide-react";
import {
  startTransition,
  useCallback,
  useLayoutEffect,
  useRef,
  useState,
  type RefObject,
} from "react";

import type { CommandIcon } from "../core/types";
import { shouldSyncSearchInput } from "../features/launcher/searchInput";
import { useI18n } from "../i18n";
import { BrandMark } from "./BrandMark";
import { CommandGlyphIcon } from "./CommandGlyph";

interface SearchBarProps {
  value: string;
  shortcut: string;
  inputRef: RefObject<HTMLInputElement | null>;
  onChange: (value: string) => void;
  onClear: () => void;
  onDragStart: () => void;
  onActivateScope?: (value: string) => boolean;
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
  onActivateScope,
  scope = null,
  onClearScope,
}: SearchBarProps) {
  const { t } = useI18n();
  const [hasDraft, setHasDraft] = useState(Boolean(value));
  const draft = useRef(value);
  const composing = useRef(false);
  const scopeKey = scope?.keyword ?? null;
  const previousScopeKey = useRef(scopeKey);

  const resetCaret = useCallback(() => {
    window.requestAnimationFrame(() => {
      const input = inputRef.current;
      if (!input) return;
      input.focus({ preventScroll: true });
      input.setSelectionRange(0, 0);
    });
  }, [inputRef]);

  useLayoutEffect(() => {
    const input = inputRef.current;
    if (!input) return;
    const scopeChanged = previousScopeKey.current !== scopeKey;
    previousScopeKey.current = scopeKey;
    const inputFocused = document.activeElement === input;
    if (
      shouldSyncSearchInput({
        externalValue: value,
        currentValue: input.value,
        scopeChanged,
        inputFocused,
      })
    ) {
      input.value = value;
      draft.current = value;
      setHasDraft(Boolean(value));
      if (scopeChanged) resetCaret();
    }
  }, [inputRef, resetCaret, scopeKey, value]);

  const clearDraft = useCallback(() => {
    const input = inputRef.current;
    if (input) input.value = "";
    draft.current = "";
    setHasDraft(false);
  }, [inputRef]);

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
        <button
          type="button"
          className="search-scope"
          title={t("search.clearScope")}
          aria-label={`${t("search.removeScope")}: ${scope.title}`}
          onClick={() => {
            clearDraft();
            onClearScope?.();
            resetCaret();
          }}
        >
          <CommandGlyphIcon icon={scope.icon} size={13} strokeWidth={1.9} />
          <span className="search-scope-label">{scope.title}</span>
        </button>
      ) : null}

      <input
        ref={inputRef}
        className="search-input"
        defaultValue={value}
        onChange={(event) => {
          const next = event.currentTarget.value;
          draft.current = next;
          setHasDraft(next.length > 0);
          if (!composing.current) {
            startTransition(() => onChange(next));
          }
        }}
        onCompositionStart={() => {
          composing.current = true;
        }}
        onCompositionEnd={(event) => {
          composing.current = false;
          const next = event.currentTarget.value;
          draft.current = next;
          setHasDraft(next.length > 0);
          startTransition(() => onChange(next));
        }}
        onKeyDown={(event) => {
          if (event.key === "Tab" && !event.shiftKey) {
            event.preventDefault();
            if (!scope && onActivateScope?.(draft.current)) {
              clearDraft();
            }
            return;
          }
          if (scope && event.key === "Backspace" && draft.current === "") {
            event.preventDefault();
            clearDraft();
            onClearScope?.();
            resetCaret();
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

      {hasDraft ? (
        <button
          type="button"
          className="clear-button"
          onClick={() => {
            clearDraft();
            onClear();
            resetCaret();
          }}
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
