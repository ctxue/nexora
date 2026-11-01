import { AnimatePresence, MotionConfig, motion } from "motion/react";
import { useCallback, useEffect, useRef, useState } from "react";

import { AnimatedHeight } from "./components/AnimatedHeight";
import { Footer } from "./components/Footer";
import { QuickActions } from "./components/QuickActions";
import { RecentList } from "./components/RecentList";
import { ResultList } from "./components/ResultList";
import { SearchBar } from "./components/SearchBar";
import type { AppEntry } from "./core/types";
import { useManualApps } from "./features/apps/useManualApps";
import { useCommands } from "./features/commands/useCommands";
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
  const [indexedApps, setIndexedApps] = useState<AppEntry[]>([]);

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
  });

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
    confirmSelection,
    hide,
    moveSelection,
    openSettings,
    query,
    scope,
    setQuery,
  ]);

  useEffect(() => {
    if (scope) focusSearchInput();
  }, [focusSearchInput, scope]);

  const compactMode =
    settings.windowMode === "compact" && query.trim().length === 0;

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
              if (scope) {
                setQuery(`${scope.keyword} ${value}`);
              } else {
                setQuery(value);
              }
            }}
            onDragStart={() => void platform.startDragging()}
            onClear={() => {
              if (scope) {
                setQuery(`${scope.keyword} `);
              } else {
                clearQuery();
              }
              inputRef.current?.focus();
            }}
            scope={scope}
            onClearScope={() => {
              setQuery("");
              inputRef.current?.focus();
            }}
          />

          {!compactMode ? (
            <>
              <div className="search-divider" />

              <AnimatedHeight>
                {query.trim() ? (
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
