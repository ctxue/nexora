import { AnimatePresence, MotionConfig, motion } from "motion/react";
import {
  useCallback,
  useEffect,
  useMemo,
  useReducer,
  useRef,
  useState,
} from "react";

import { AnimatedHeight } from "./components/AnimatedHeight";
import { ActionPanel } from "./components/ActionPanel";
import { ClipboardList } from "./components/ClipboardList";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { Footer } from "./components/Footer";
import { QuickActions } from "./components/QuickActions";
import { PreviewPanel } from "./components/PreviewPanel";
import { RecentList } from "./components/RecentList";
import { ResultList } from "./components/ResultList";
import { SearchBar } from "./components/SearchBar";
import type { AppEntry, LauncherItem } from "./core/types";
import { useManualApps } from "./features/apps/useManualApps";
import { useCommands } from "./features/commands/useCommands";
import { useClipboard } from "./features/clipboard/useClipboard";
import { filterClipboardEntries } from "./features/clipboard/filter";
import { clipboardUiReducer, INITIAL_CLIPBOARD_UI } from "./features/clipboard/ui";
import { useIndexFolders } from "./features/indexes/useIndexFolders";
import { useLauncher } from "./features/launcher/useLauncher";
import { buildResultActions } from "./features/actions/registry";
import type { ResultActionDescriptor } from "./features/actions/types";
import { useResultActions } from "./features/actions/useResultActions";
import { usePlugins } from "./features/plugins/usePlugins";
import { useSettings } from "./features/settings/useSettings";
import {
  requestSettingsSection,
  type SettingsSectionId,
} from "./features/settings/navigation";
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
  const [clipboardUi, dispatchClipboardUi] = useReducer(
    clipboardUiReducer,
    INITIAL_CLIPBOARD_UI,
  );
  const [clearingClipboard, setClearingClipboard] = useState(false);
  const [actionTarget, setActionTarget] = useState<LauncherItem | null>(null);
  const [actionIndex, setActionIndex] = useState(0);
  const [previewPath, setPreviewPath] = useState<string | null>(null);
  const clipboardOpen = clipboardUi.open;
  const clipboardSelected = clipboardUi.selectedIndex;

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

  const openSettings = useCallback((section?: SettingsSectionId) => {
    requestSettingsSection(section ?? null);
    void platform.openSettingsWindow();
  }, []);

  const openClipboard = useCallback(() => {
    dispatchClipboardUi({ type: "open" });
  }, []);

  const {
    query,
    scope,
    argument,
    activateScope,
    setQuery,
    groups,
    selectedItem,
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

  const resultActions = useResultActions(actionTarget);

  const openActionPanel = useCallback((item: LauncherItem | null) => {
    if (!item || buildResultActions(item).length === 0) return;
    setActionTarget(item);
    setActionIndex(0);
  }, []);

  const closeActionPanel = useCallback(() => {
    setActionTarget(null);
    window.requestAnimationFrame(focusSearchInput);
  }, [focusSearchInput]);

  const executeResultAction = useCallback(
    async (action: ResultActionDescriptor) => {
      const item = actionTarget;
      if (!item) return;
      setActionTarget(null);

      try {
        switch (action.id) {
          case "copy-path":
            if (!item.path) return;
            await platform.copyClipboard(item.path);
            showToast(t("toast.resultCopied"));
            window.requestAnimationFrame(focusSearchInput);
            return;
          case "reveal-path":
            if (!item.path) return;
            await platform.revealPath(item.path);
            hide();
            return;
          case "open-with":
            if (!item.path) return;
            await platform.openWith(item.path);
            hide();
            return;
          case "run-as-admin":
            if (!item.path) return;
            await platform.runAsAdmin(item.path);
            hide();
            return;
          case "preview-file":
            if (item.path) setPreviewPath(item.path);
            return;
          case "activate-window":
          case "minimize-window":
          case "close-window":
            if (!item.windowId) return;
            await platform.controlWindow(
              item.windowId,
              action.id === "activate-window"
                ? "activate"
                : action.id === "minimize-window"
                  ? "minimize"
                  : "close",
            );
            hide();
            return;
        }
      } catch {
        showToast(t("toast.actionFailed"));
        window.requestAnimationFrame(focusSearchInput);
      }
    },
    [actionTarget, focusSearchInput, hide, showToast, t],
  );

  const visibleClipboardEntries = useMemo(
    () =>
      clipboardOpen ? filterClipboardEntries(clipboard.entries, query) : [],
    [clipboard.entries, clipboardOpen, query],
  );

  const closeClipboard = useCallback(() => {
    dispatchClipboardUi({ type: "close" });
    clearQuery();
  }, [clearQuery]);

  const clearClipboardHistory = useCallback(async () => {
    if (clearingClipboard) return;
    setClearingClipboard(true);
    try {
      await clipboard.clear();
      dispatchClipboardUi({ type: "confirm-clear" });
      window.requestAnimationFrame(focusSearchInput);
    } catch {
      dispatchClipboardUi({ type: "cancel-clear" });
      showToast(t("toast.actionFailed"));
    } finally {
      setClearingClipboard(false);
    }
  }, [clearingClipboard, clipboard, focusSearchInput, showToast, t]);

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
      dispatchClipboardUi({
        type: "move",
        delta,
        itemCount: visibleClipboardEntries.length,
      });
    },
    [visibleClipboardEntries.length],
  );

  useEffect(() => {
    dispatchClipboardUi({
      type: "clamp",
      itemCount: visibleClipboardEntries.length,
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
        dispatchClipboardUi({ type: "close" });
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

      if (previewPath) {
        if (event.key === "Escape") {
          event.preventDefault();
          setPreviewPath(null);
          window.requestAnimationFrame(focusSearchInput);
        } else {
          event.preventDefault();
        }
        return;
      }

      if (actionTarget) {
        if (event.key === "ArrowDown") {
          event.preventDefault();
          setActionIndex((current) =>
            Math.min(current + 1, Math.max(0, resultActions.length - 1)),
          );
        } else if (event.key === "ArrowUp") {
          event.preventDefault();
          setActionIndex((current) => Math.max(0, current - 1));
        } else if (event.key === "Enter") {
          event.preventDefault();
          const action = resultActions[actionIndex];
          if (action) void executeResultAction(action);
        } else if (event.key === "Escape") {
          event.preventDefault();
          closeActionPanel();
        }
        return;
      }

      if ((event.ctrlKey || event.metaKey) && event.key === ",") {
        event.preventDefault();
        openSettings();
        return;
      }

      if ((event.ctrlKey || event.metaKey) && event.key === "Enter") {
        event.preventDefault();
        openActionPanel(selectedItem);
        return;
      }

      if ((event.ctrlKey || event.metaKey) && event.code === "Space") {
        event.preventDefault();
        if (selectedItem?.kind === "file" && selectedItem.path) {
          setPreviewPath(selectedItem.path);
        }
        return;
      }

      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        inputRef.current?.focus();
        inputRef.current?.select();
        return;
      }

      if (clipboardOpen) {
        if (clipboardUi.clearConfirmationOpen) return;

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
    actionIndex,
    actionTarget,
    clearQuery,
    clipboardOpen,
    clipboardSelected,
    clipboardUi.clearConfirmationOpen,
    closeClipboard,
    confirmSelection,
    copyClipboardEntry,
    closeActionPanel,
    executeResultAction,
    focusSearchInput,
    hide,
    moveSelection,
    moveClipboardSelection,
    openActionPanel,
    openSettings,
    previewPath,
    query,
    resultActions,
    selectedItem,
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
              setActionTarget(null);
              if (clipboardOpen) {
                setQuery(value);
              } else if (scope) {
                setQuery(`${scope.keyword}\t${value}`);
              } else {
                setQuery(value);
              }
            }}
            onDragStart={() => void platform.startDragging()}
            onActivateScope={activateScope}
            onClear={() => {
              if (clipboardOpen) {
                clearQuery();
              } else if (scope) {
                setQuery(`${scope.keyword}\t`);
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

              <AnimatedHeight animate={!query.trim() || clipboardOpen}>
                {clipboardOpen ? (
                  <ClipboardList
                    entries={visibleClipboardEntries}
                    selectedIndex={clipboardSelected}
                    enabled={settings.clipboardHistory}
                    onSelect={(index) =>
                      dispatchClipboardUi({ type: "select", index })
                    }
                    onCopy={(entry) => void copyClipboardEntry(entry)}
                    onTogglePin={(entry) => void clipboard.togglePin(entry.id)}
                    onRemove={(entry) => void clipboard.remove(entry.id)}
                    onClear={() =>
                      dispatchClipboardUi({ type: "request-clear" })
                    }
                    onOpenSettings={() => openSettings("features")}
                  />
                ) : query.trim() ? (
                  <ResultList
                    groups={groups}
                    query={query}
                    icons={icons}
                    selectedIndex={selectedIndex}
                    onSelect={setSelectedIndex}
                    onExecute={execute}
                    onActions={openActionPanel}
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
                showActions={Boolean(
                  selectedItem &&
                    (selectedItem.path || selectedItem.windowId),
                )}
              />
            </>
          ) : null}

          {clipboardUi.clearConfirmationOpen ? (
            <ConfirmDialog
              title={t("clipboard.confirmClearTitle")}
              description={t("clipboard.confirmClear")}
              cancelLabel={t("common.cancel")}
              confirmLabel={t("clipboard.clear")}
              tone="danger"
              busy={clearingClipboard}
              onCancel={() => {
                dispatchClipboardUi({ type: "cancel-clear" });
                window.requestAnimationFrame(focusSearchInput);
              }}
              onConfirm={() => void clearClipboardHistory()}
            />
          ) : null}

          {actionTarget ? (
            <ActionPanel
              item={actionTarget}
              actions={resultActions}
              selectedIndex={actionIndex}
              icons={icons}
              onSelect={setActionIndex}
              onRun={(action) => void executeResultAction(action)}
              onClose={closeActionPanel}
            />
          ) : null}

          {previewPath ? (
            <PreviewPanel
              path={previewPath}
              onClose={() => {
                setPreviewPath(null);
                window.requestAnimationFrame(focusSearchInput);
              }}
            />
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
