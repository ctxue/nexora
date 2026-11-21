import { ChevronRight, Folder, HardDrive, X } from "lucide-react";
import {
  startTransition,
  useCallback,
  useLayoutEffect,
  useRef,
  useState,
  type RefObject,
} from "react";

import type { CommandIcon } from "../core/types";
import { parseSearchQuery } from "../core/query";
import { hasReachedDragThreshold } from "../features/launcher/drag";
import { searchPlaceholderFor } from "../features/launcher/searchPlaceholder";
import { shouldSyncSearchInput } from "../features/launcher/searchInput";
import { useI18n } from "../i18n";
import { CommandGlyphIcon } from "./CommandGlyph";
import { Badge } from "./ui/badge";

interface SearchBarProps {
  value: string;
  inputRef: RefObject<HTMLInputElement | null>;
  showPlaceholder: boolean;
  placeholderText: string;
  onChange: (value: string) => void;
  onClear: () => void;
  onDragStart: () => void;
  onActivateScope?: (value: string) => boolean;
  scope?: { title: string; keyword: string; icon: CommandIcon } | null;
  onClearScope?: () => void;
}

export function SearchBar({
  value,
  inputRef,
  showPlaceholder,
  placeholderText,
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
  const dragOrigin = useRef<{
    pointerId: number;
    x: number;
    y: number;
  } | null>(null);
  const scopeKey = scope?.keyword ?? null;
  const previousScopeKey = useRef(scopeKey);
  const placeholderCopy = searchPlaceholderFor({
    enabled: showPlaceholder,
    custom: placeholderText,
    fallback: t("search.placeholder"),
    scoped: scope
      ? t("search.scopedPlaceholder", { name: scope.title })
      : null,
  });
  const pathRoot = parseSearchQuery(value).pathRoot;
  const pathCrumbs = pathRoot
    ? pathRoot
        .replace(/\\+$/u, "")
        .split("\\")
        .filter(Boolean)
        .map((crumb, index) => (index === 0 ? crumb.toUpperCase() : crumb))
    : [];

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
    <div className="search-header">
      <div
        className="search-bar"
      onPointerDown={(event) => {
        if (event.button !== 0) return;
        dragOrigin.current = {
          pointerId: event.pointerId,
          x: event.clientX,
          y: event.clientY,
        };
      }}
      onPointerMove={(event) => {
        const origin = dragOrigin.current;
        if (!origin || origin.pointerId !== event.pointerId) return;
        if (
          !hasReachedDragThreshold(
            event.clientX - origin.x,
            event.clientY - origin.y,
          )
        ) {
          return;
        }
        dragOrigin.current = null;
        event.preventDefault();
        onDragStart();
      }}
      onPointerUp={() => {
        dragOrigin.current = null;
      }}
      onPointerCancel={() => {
        dragOrigin.current = null;
      }}
      >
      {scope ? (
        <Badge
          variant="secondary"
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
        </Badge>
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
        placeholder={placeholderCopy.placeholder}
        aria-label={placeholderCopy.label}
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
      </div>
      {pathCrumbs.length > 0 ? (
        <div className="path-breadcrumb" aria-label="Current path">
          {pathCrumbs.map((crumb, index) => (
            <span className="path-crumb" key={`${crumb}-${index}`}>
              {index > 0 ? (
                <ChevronRight
                  className="path-chevron"
                  size={10}
                  strokeWidth={1.8}
                />
              ) : null}
              {index === 0 ? (
                <HardDrive size={11} strokeWidth={1.8} />
              ) : (
                <Folder size={11} strokeWidth={1.8} />
              )}
              <span>{crumb}</span>
            </span>
          ))}
        </div>
      ) : null}
    </div>
  );
}
