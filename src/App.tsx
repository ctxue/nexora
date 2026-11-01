import { AnimatePresence, MotionConfig, motion } from "motion/react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { AnimatedHeight } from "./components/AnimatedHeight";
import { ClipboardList } from "./components/ClipboardList";
import { Footer } from "./components/Footer";
import { QuickActions } from "./components/QuickActions";
import { RecentList } from "./components/RecentList";
import { ResultList } from "./components/ResultList";
import { SearchBar } from "./components/SearchBar";
import type { AppEntry } from "./core/types";
import { useManualApps } from "./features/apps/useManualApps";
import { useCommands } from "./features/commands/useCommands";
import { useClipboard } from "./features/clipboard/useClipboard";
import { filterClipboardEntries } from "./features/clipboard/filter";
import { useIndexFolders } from "./features/indexes/useIndexFolders";
import { useLauncher } from "./features/launcher/useLauncher";
import { usePlugins } from "./features/plugins/usePlugins";
import { useSettings } from "./features/settings/useSettings";
import type { Settings } from "./features/settings/types";
import { I18nProvider, useI18n } from "./i18n";
import { platform } from "./platform/tauri";

export default function App() {
  const { settings } = useSettings();

  return (
    <I18nProvider language={settings.language}>
      <Launcher settings={settings} />
    </I18nProvider>
  );
}

function Launcher({ settings }: { settings: Settings }) {
  const { t } = useI18n();
  const inputRef = useRef<HTMLInputElement>(null);
  const { commands } = useCommands();
  const { apps: manualApps } = useManualApps();
  const { folders: indexFolders, updatedAt: indexUpdatedAt } = useIndexFolders();
  const { plugins, disabled: disabledPlugins } = usePlugins();
  const clipboard = useClipboard();
  const [indexedApps, setIndexedApps] = useState<AppEntry[]>([]);
  const [clipboardOpen, setClipboardOpen] = useState(false);
  const [clipboardSelected, setClipboardSelected] = useState(0);

  useEffect(() => {
    if (indexFolders.length === 0) {
      setIndexedApps([]);
      return;
    }

    let cancelled = false;
    void platform
      .scanIndexFolders(indexFolders.map((folder) => folder.path))
      .then((apps) => {
        if (!cancelled) setIndexedApps(apps);
      })
      .catch(() => {
        if (!cancelled) setIndexedApps([]);
      });

    return () => {
      cancelled = true;
    };
  }, [indexFolders, indexUpdatedAt]);

  const focusSearchInput = useCallback(() => {
    const input = inputRef.current;
    if (!input) return;
    input.focus({ preventScroll: true });
    input.select();
  }, []);

  const openSettings = useCallback(() => {
    void platform.openSettingsWindow();
  }, []);

  const openClipboard = useCallback(() => {
    setClipboardOpen(true);
    setClipboardSelected(0);
  }, []);

  const {
    query,
    scope,
    argument,
    setQuery,
    groups,
    selectedIndex,
    setSelectedIndex,
    moveSelection,
    confirmSelection,
    execute,
    startItems,
    recents,
    removeRecent,
    clearRecents,
    icons,
    hide,
    clearQuery,
    toast,
    indexing,
    fileCount,
    showNonce,
    showToast,
  } = useLauncher({
    settings,
    commands,
    manualApps,
    indexedApps,
    plugins,
    disabledPlugins,
    onOpenSettings: openSettings,
    onOpenClipboard: openClipboard,
    searchSuspended: clipboardOpen,
  });

  const visibleClipboardEntries = useMemo(
    () => filterClipboardEntries(clipboard.entries, query),
    [clipboard.entries, query],
  );

  const closeClipboard = useCallback(() => {
    setClipboardOpen(false);
    setClipboardSelected(0);
    clearQuery();
  }, [clearQuery]);

  const copyClipboardEntry = useCallback(
    async (entry: (typeof visibleClipboardEntries)[number]) => {
      try {
        await clipboard.copy(entry.text);
        closeClipboard();
        hide();
      } catch {
        showToast(t("toast.actionFailed"));
      }
    },
    [clipboard, closeClipboard, hide, showToast, t],
  );

  const moveClipboardSelection = useCallback(
    (delta: number) => {
      if (visibleClipboardEntries.length === 0) return;
      setClipboardSelected((current) => {
        const next = current + delta;
        return Math.max(0, Math.min(next, visibleClipboardEntries.length - 1));
      });
    },
    [visibleClipboardEntries.length],
  );

  useEffect(() => {
    setClipboardSelected((current) => {
      if (visibleClipboardEntries.length === 0) return 0;
      return Math.min(current, visibleClipboardEntries.length - 1);
    });
  }, [visibleClipboardEntries.length]);

  useEffect(() => {
    void platform
      .setClipboardCapture(settings.clipboardHistory)
      .catch(() => {});
  }, [settings.clipboardHistory]);

  useEffect(() => {
    void platform.setTrayLanguage(settings.language).catch(() => {
      // The tray is optional; keep the launcher usable if it is unavailable.
    });
  }, [settings.language]);

  useEffect(() => {
    void platform.setGlobalShortcut(settings.shortcut).catch(() => {
      showToast(t("toast.shortcutFailed"));
      void platform.openSettingsWindow();
    });
    // The saved shortcut is applied once on startup; later changes are made
    // from the separate settings window and synced through localStorage.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    // Focus immediately, then once more after the native window has finished
    // activating. This prevents the Windows beep when the first characters
    // arrive before the webview has an active editable element.
    focusSearchInput();
    const frame = window.requestAnimationFrame(focusSearchInput);
    const timer = window.setTimeout(focusSearchInput, 60);
    return () => {
      window.cancelAnimationFrame(frame);
      window.clearTimeout(timer);
    };
  }, [focusSearchInput, showNonce]);

  useEffect(() => {
    window.addEventListener("focus", focusSearchInput);
    return () => window.removeEventListener("focus", focusSearchInput);
  }, [focusSearchInput]);

  useEffect(() => {
    let mounted = true;
    let unlisten: (() => void) | undefined;
    void platform.onLauncherHidden(() => {
      if (mounted) {
        setClipboardOpen(false);
        setClipboardSelected(0);
      }
    }).then((dispose) => {
      if (mounted) unlisten = dispose;
      else dispose();
    });
    return () => {
      mounted = false;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.isComposing || event.keyCode === 229) return;

      if ((event.ctrlKey || event.metaKey) && event.key === ",") {
        event.preventDefault();
        openSettings();
        return;
      }

      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        inputRef.current?.focus();
        inputRef.current?.select();
        return;
      }

      if (clipboardOpen) {
        if (event.key === "ArrowDown") {
          event.preventDefault();
          moveClipboardSelection(1);
          return;
        }

        if (event.key === "ArrowUp") {
          event.preventDefault();
          moveClipboardSelection(-1);
          return;
        }

        if (event.key === "Enter") {
          event.preventDefault();
          const entry = visibleClipboardEntries[clipboardSelected];
          if (entry) void copyClipboardEntry(entry);
          return;
        }

        if (event.key === "Escape") {
          event.preventDefault();
          if (query.trim()) {
            clearQuery();
          } else {
            closeClipboard();
          }
          inputRef.current?.focus();
          return;
        }
      }

      if (event.key === "ArrowDown") {
        event.preventDefault();
        moveSelection(1);
        return;
      }

      if (event.key === "ArrowUp") {
        event.preventDefault();
        moveSelection(-1);
        return;
      }

      if (event.key === "Enter") {
        event.preventDefault();
        void confirmSelection();
        return;
      }

      if (event.key === "Escape") {
        event.preventDefault();
        if (scope) {
          setQuery("");
          inputRef.current?.focus();
        } else if (query.trim()) {
          clearQuery();
          inputRef.current?.focus();
        } else {
          hide();
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [
    clearQuery,
    clipboardOpen,
    clipboardSelected,
    closeClipboard,
    confirmSelection,
    copyClipboardEntry,
    hide,
    moveSelection,
    moveClipboardSelection,
    openSettings,
    query,
    scope,
    setQuery,
    visibleClipboardEntries,
  ]);

  useEffect(() => {
    if (scope) focusSearchInput();
  }, [focusSearchInput, scope]);

  const compactMode =
    settings.windowMode === "compact" &&
    query.trim().length === 0 &&
    !clipboardOpen;

  const activeScope = clipboardOpen
    ? {
        title: t("clipboard.history"),
        keyword: "clipboard",
        icon: "clipboard" as const,
      }
    : scope;

  return (
    <MotionConfig reducedMotion="user">
      <main
        className="shell"
        onPointerDown={(event) => {
          const target = event.target as Element;
          if (!target.closest(".launcher-card")) {
            hide();
          }
        }}
      >
        <section
          className="launcher-card"
          role="dialog"
          aria-label="NEXORA"
        >
          <SearchBar
            value={scope ? argument : query}
            shortcut={settings.shortcut}
            inputRef={inputRef}
            onChange={(value) => {
              if (clipboardOpen) {
                setQuery(value);
              } else if (scope) {
                setQuery(`${scope.keyword} ${value}`);
              } else {
                setQuery(value);
              }
            }}
            onDragStart={() => void platform.startDragging()}
            onClear={() => {
              if (clipboardOpen) {
                clearQuery();
              } else if (scope) {
                setQuery(`${scope.keyword} `);
              } else {
                clearQuery();
              }
              inputRef.current?.focus();
            }}
            scope={activeScope}
            onClearScope={() => {
              if (clipboardOpen) closeClipboard();
              else setQuery("");
              inputRef.current?.focus();
            }}
          />

          {!compactMode ? (
            <>
              <div className="search-divider" />

              <AnimatedHeight>
                {clipboardOpen ? (
                  <ClipboardList
                    entries={visibleClipboardEntries}
                    selectedIndex={clipboardSelected}
                    onSelect={setClipboardSelected}
                    onCopy={(entry) => void copyClipboardEntry(entry)}
                    onTogglePin={(entry) => void clipboard.togglePin(entry.id)}
                    onRemove={(entry) => void clipboard.remove(entry.id)}
                    onClear={() => {
                      if (window.confirm(t("clipboard.confirmClear"))) {
                        void clipboard.clear();
                      }
                    }}
                  />
                ) : query.trim() ? (
                  <ResultList
                    groups={groups}
                    query={query}
                    icons={icons}
                    selectedIndex={selectedIndex}
                    onSelect={setSelectedIndex}
                    onExecute={execute}
                  />
                ) : settings.showRecent && recents.length > 0 ? (
                  <RecentList
                    items={startItems}
                    icons={icons}
                    selectedIndex={selectedIndex}
                    onSelect={setSelectedIndex}
                    onOpen={execute}
                    onRemove={removeRecent}
                    onClear={clearRecents}
                  />
                ) : (
                  <QuickActions
                    items={startItems}
                    selectedIndex={selectedIndex}
                    onSelect={setSelectedIndex}
                    onOpen={execute}
                  />
                )}
              </AnimatedHeight>

              <Footer
                indexing={indexing}
                fileCount={fileCount}
                onSettings={openSettings}
              />
            </>
          ) : null}
        </section>

        <AnimatePresence>
          {toast ? (
            <motion.div
              className="toast"
              initial={{ opacity: 0, y: 8, scale: 0.98 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, y: 6, scale: 0.98 }}
              transition={{ duration: 0.16, ease: [0.16, 1, 0.3, 1] }}
              role="status"
            >
              {toast}
            </motion.div>
          ) : null}
        </AnimatePresence>
      </main>
    </MotionConfig>
  );
}
