import {
  useCallback,
  useDeferredValue,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import type { UnlistenFn } from "@tauri-apps/api/event";

import {
  acceptsArgument,
  buildArguments,
  buildTarget,
  buildUrl,
  COMMAND_ICON_CHOICES,
  commandHost,
  findArgumentCommand,
  parseQuery,
  scoreCustomCommand,
  type CustomCommand,
  type TemplateContext,
} from "../../core/commands";
import { evaluateSmartQuery } from "../../core/calculator";
import { hasSearchFilters, parseSearchQuery } from "../../core/query";
import { buildGroups, commandToItem } from "../../core/search";
import { searchWindows } from "../../core/windows";
import type {
  AppEntry,
  CommandIcon,
  FileEntry,
  LauncherCommand,
  LauncherItem,
  QuickPaths,
  RecentItem,
  ResultGroup,
  WindowEntry,
} from "../../core/types";
import {
  platform,
  type DriveInfo,
  type PluginItem,
  type PluginManifest,
} from "../../platform/tauri";
import {
  mergeIndexedApps,
  mergeManualApps,
  type ManualApp,
} from "../apps/storage";
import type { Settings } from "../settings/types";
import { useI18n } from "../../i18n";

const RECENTS_KEY = "nexora.recents.v1";
const FILE_SEARCH_DEBOUNCE_MS = 100;
const QUICK_ACTION_IDS = [
  "open-downloads",
  "open-computer",
  "open-recycle-bin",
];

function loadRecents(): RecentItem[] {
  try {
    const raw = window.localStorage.getItem(RECENTS_KEY);
    if (!raw) return [];

    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];

    return parsed
      .filter((item): item is RecentItem => {
        if (!item || typeof item !== "object") return false;
        const candidate = item as Partial<RecentItem>;
        return (
          typeof candidate.id === "string" &&
          typeof candidate.title === "string" &&
          (candidate.kind === "app" ||
            candidate.kind === "file" ||
            candidate.kind === "command")
        );
      })
      .slice(0, 8);
  } catch {
    return [];
  }
}

function saveRecents(items: RecentItem[]) {
  try {
    window.localStorage.setItem(RECENTS_KEY, JSON.stringify(items));
  } catch {
    // Recents are a convenience; a full storage quota must not break search.
  }
}

interface LauncherOptions {
  settings: Settings;
  commands: CustomCommand[];
  manualApps: ManualApp[];
  indexedApps: AppEntry[];
  plugins: PluginManifest[];
  disabledPlugins: string[];
  onOpenSettings: () => void;
  onOpenClipboard: () => void;
  searchSuspended: boolean;
}

export function useLauncher({
  settings,
  commands: customCommands,
  manualApps,
  indexedApps,
  plugins,
  disabledPlugins,
  onOpenSettings,
  onOpenClipboard,
  searchSuspended,
}: LauncherOptions) {
  const { t } = useI18n();
  const [apps, setApps] = useState<AppEntry[]>([]);
  const [paths, setPaths] = useState<QuickPaths | null>(null);
  const [drives, setDrives] = useState<DriveInfo[]>([]);
  const [windows, setWindows] = useState<WindowEntry[]>([]);
  const [query, setQueryState] = useState("");
  const [fileResults, setFileResults] = useState<FileEntry[]>([]);
  const [fileCount, setFileCount] = useState<number | null>(null);
  const [indexing, setIndexing] = useState(true);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [toast, setToast] = useState<string | null>(null);
  const [showNonce, setShowNonce] = useState(0);
  const [recents, setRecents] = useState<RecentItem[]>(loadRecents);
  const [icons, setIcons] = useState<Record<string, string>>({});

  const toastTimer = useRef<number | null>(null);
  const searchRequest = useRef(0);
  const requestedIcons = useRef<Set<string>>(new Set());

  const showToast = useCallback((message: string) => {
    if (toastTimer.current) {
      window.clearTimeout(toastTimer.current);
    }
    setToast(message);
    toastTimer.current = window.setTimeout(() => {
      setToast(null);
      toastTimer.current = null;
    }, 2200);
  }, []);

  const clearQuery = useCallback(() => {
    setQueryState("");
    setFileResults([]);
    setSelectedIndex(0);
  }, []);

  const setQuery = useCallback((value: string) => {
    setQueryState(value);
    setSelectedIndex(0);
  }, []);

  const hide = useCallback(() => {
    clearQuery();
    void platform.hide().catch(() => {
      // The window may already be hidden.
    });
  }, [clearQuery]);

  const runPath = useCallback((path: string) => platform.openPath(path), []);

  const parsed = useMemo(
    () => parseQuery(query, customCommands),
    [customCommands, query],
  );
  const deferredQuery = useDeferredValue(query);
  const deferredParsed = useMemo(
    () => parseQuery(deferredQuery, customCommands),
    [customCommands, deferredQuery],
  );
  const searchQuery = deferredParsed.argument;
  const searchFilters = useMemo(
    () => parseSearchQuery(searchQuery),
    [searchQuery],
  );
  const scope = parsed.scope;
  const argument = scope ? parsed.argument : "";

  const allApps = useMemo(
    () => mergeManualApps(mergeIndexedApps(apps, indexedApps), manualApps),
    [apps, indexedApps, manualApps],
  );

  const templateContext = useCallback(async (): Promise<TemplateContext> => {
    const [clipboard, selected] = await Promise.all([
      platform.readClipboardText().catch(() => ""),
      platform.getSelectedText().catch(() => ""),
    ]);
    return {
      clipboard,
      selected,
      date: new Date().toLocaleDateString("sv-SE"),
    };
  }, []);

  const runCustomCommand = useCallback(
    async (command: CustomCommand, value: string) => {
      const action = command.action;
      const context = await templateContext();
      switch (action.kind) {
        case "web-search":
          return platform.openUrl(
            buildUrl(action.template, value, context),
            action.browser,
            action.newWindow,
          );
        case "open-url":
          return platform.openUrl(
            buildUrl(action.target, value, context),
            action.browser,
            action.newWindow,
          );
        case "open-path":
          return platform.openPath(
            buildTarget(action.target, value, context),
          );
        case "launch":
          return platform.launchProgram(
            buildTarget(action.target, value, context),
            buildArguments(action.args, value, context),
          );
      }
    },
    [templateContext],
  );

  const enterScope = useCallback((command: CustomCommand) => {
    setQueryState(`${command.keyword}\t`);
    setSelectedIndex(0);
  }, []);

  const activateScope = useCallback((value = query) => {
    if (scope) return false;
    const command = findArgumentCommand(value, customCommands);
    if (!command) return false;
    enterScope(command);
    return true;
  }, [customCommands, enterScope, query, scope]);

  const commandItem = useCallback(
    (command: CustomCommand): LauncherItem => {
      const host = commandHost(command);
      const needsArgument = acceptsArgument(command);

      return {
        id: `custom:${command.id}`,
        kind: "command",
        title: command.title,
        subtitle: needsArgument
          ? t("commands.item.keywordHint", { keyword: command.keyword })
          : command.subtitle || host,
        commandIcon: command.icon,
        keepOpen: needsArgument,
        run: () =>
          needsArgument ? enterScope(command) : runCustomCommand(command, ""),
      };
    },
    [enterScope, runCustomCommand, t],
  );

  const scopedItem = useCallback(
    (command: CustomCommand, value: string): LauncherItem => {
      const host = commandHost(command);
      const title = value
        ? command.action.kind === "web-search"
          ? t("commands.item.searchTitle", { query: value })
          : t("commands.item.runTitle", { query: value })
        : command.title;
      const subtitle = value
        ? [command.title, host].filter(Boolean).join(" · ")
        : t("commands.item.typeQuery");

      return {
        id: `custom:${command.id}`,
        kind: "command",
        title,
        subtitle,
        commandIcon: command.icon,
        keepOpen: false,
        run: () => runCustomCommand(command, value),
      };
    },
    [runCustomCommand, t],
  );

  const enabledPlugins = useMemo(
    () => plugins.filter((plugin) => !disabledPlugins.includes(plugin.id)),
    [disabledPlugins, plugins],
  );

  const [pluginResults, setPluginResults] = useState<{
    plugin: PluginManifest;
    items: PluginItem[];
  } | null>(null);
  const pluginRequest = useRef(0);

  useEffect(() => {
    pluginRequest.current += 1;
    const requestId = pluginRequest.current;

    if (scope || searchSuspended || hasSearchFilters(searchFilters)) {
      setPluginResults(null);
      return;
    }

    const trimmed = searchQuery.trim();
    if (!trimmed) {
      setPluginResults(null);
      return;
    }

    const spaceIndex = trimmed.search(/\s/u);
    const keyword = (
      spaceIndex === -1 ? trimmed : trimmed.slice(0, spaceIndex)
    ).toLowerCase();
    const plugin = enabledPlugins.find((entry) =>
      entry.keywords.some((value) => value.toLowerCase() === keyword),
    );
    if (!plugin) {
      setPluginResults(null);
      return;
    }

    const argument =
      spaceIndex === -1 ? "" : trimmed.slice(spaceIndex + 1).trim();

    const timer = window.setTimeout(() => {
      void platform
        .searchPlugin(plugin.id, argument, keyword)
        .then((items) => {
          if (pluginRequest.current === requestId) {
            setPluginResults({ plugin, items });
          }
        })
        .catch(() => {
          if (pluginRequest.current === requestId) {
            setPluginResults(null);
          }
        });
    }, 120);

    return () => window.clearTimeout(timer);
  }, [enabledPlugins, scope, searchFilters, searchQuery, searchSuspended]);

  const runPluginAction = useCallback(
    async (action: PluginItem["action"]) => {
      if (!action) return;
      if (action.type === "openUrl") {
        if (!/^https?:\/\//iu.test(action.url)) {
          throw new Error("Invalid plugin URL");
        }
        await platform.openUrl(action.url, "default", false);
        return;
      }
      await platform.openPath(action.path);
    },
    [],
  );

  const pluginToItem = useCallback(
    (plugin: PluginManifest, item: PluginItem): LauncherItem => {
      const icon =
        item.icon?.type === "glyph" &&
        COMMAND_ICON_CHOICES.includes(item.icon.value as CommandIcon)
          ? (item.icon.value as CommandIcon)
          : "terminal";

      return {
        id: `plugin:${plugin.id}:${item.id}`,
        kind: "command",
        title: item.title,
        subtitle: item.subtitle || plugin.name,
        commandIcon: icon,
        run: () => runPluginAction(item.action),
      };
    },
    [runPluginAction],
  );

  const commands = useMemo<LauncherCommand[]>(() => {
    const openPathOrExplain = (path: string | undefined, label: string) => {
      if (!path) {
        showToast(t("toast.folderUnavailable", { folder: label }));
        return;
      }
      return platform.openPath(path);
    };

    const driveKindLabel = (kind: DriveInfo["kind"]) => {
      if (kind === "removable") return t("drives.kind.removable");
      if (kind === "remote") return t("drives.kind.remote");
      if (kind === "ramdisk") return t("drives.kind.ramdisk");
      return t("drives.kind.fixed");
    };

    const driveCommands: LauncherCommand[] = drives.map((drive) => {
      const letter = drive.letter.toLowerCase();

      return {
        id: `drive:${drive.letter}`,
        title: `${drive.letter}:`,
        subtitle: driveKindLabel(drive.kind),
        keywords: [
          letter,
          `${letter}:`,
          `${letter}:/`,
          `${letter}:\\`,
          `${letter}://`,
          `${letter}盘`,
          `${letter} 盘`,
          "drive",
          "disk",
        ],
        icon: "computer",
        run: () => platform.openPath(drive.path),
      };
    });

    return [
      ...driveCommands,
      {
        id: "open-downloads",
        title: t("commands.openDownloads.title"),
        subtitle: t("commands.openDownloads.subtitle"),
        keywords: ["open", "downloads", "folder", "files", "下载"],
        icon: "downloads",
        run: () =>
          openPathOrExplain(paths?.downloads, t("commands.openDownloads.title")),
      },
      {
        id: "open-desktop",
        title: t("commands.openDesktop.title"),
        subtitle: t("commands.openDesktop.subtitle"),
        keywords: ["open", "desktop", "folder", "桌面"],
        icon: "desktop",
        run: () =>
          openPathOrExplain(paths?.desktop, t("commands.openDesktop.title")),
      },
      {
        id: "open-documents",
        title: t("commands.openDocuments.title"),
        subtitle: t("commands.openDocuments.subtitle"),
        keywords: ["open", "documents", "folder", "文档"],
        icon: "documents",
        run: () =>
          openPathOrExplain(paths?.documents, t("commands.openDocuments.title")),
      },
      {
        id: "open-computer",
        title: t("commands.openComputer.title"),
        subtitle: t("commands.openComputer.subtitle"),
        keywords: ["open", "this pc", "computer", "drives", "此电脑", "我的电脑"],
        icon: "computer",
        run: () =>
          openPathOrExplain("shell:MyComputerFolder", t("commands.openComputer.title")),
      },
      {
        id: "open-recycle-bin",
        title: t("commands.openRecycleBin.title"),
        subtitle: t("commands.openRecycleBin.subtitle"),
        keywords: ["open", "recycle bin", "trash", "deleted", "回收站"],
        icon: "trash",
        run: () =>
          openPathOrExplain(
            "shell:RecycleBinFolder",
            t("commands.openRecycleBin.title"),
          ),
      },
      {
        id: "clipboard",
        title: t("commands.clipboard.title"),
        subtitle: t("commands.clipboard.subtitle"),
        keywords: ["clipboard", "history", "paste", "copy", "剪贴板"],
        icon: "clipboard",
        run: () => {
          clearQuery();
          onOpenClipboard();
        },
      },
      {
        id: "settings",
        title: t("commands.settings.title"),
        subtitle: t("commands.settings.subtitle"),
        keywords: [
          "settings",
          "preferences",
          "shortcut",
          "theme",
          "language",
          "设置",
          "快捷键",
          "语言",
        ],
        icon: "settings",
        run: onOpenSettings,
      },
      {
        id: "refresh",
        title: t("commands.refresh.title"),
        subtitle: t("commands.refresh.subtitle"),
        keywords: ["refresh", "reindex", "index", "files", "scan", "刷新", "索引"],
        icon: "refresh",
        run: async () => {
          if (!settings.fileSearch) {
            showToast(t("toast.fileSearchOff"));
            return;
          }
          const started = await platform.startFileIndex();
          showToast(
            started ? t("toast.refreshStarted") : t("toast.refreshRunning"),
          );
        },
      },
      {
        id: "quit",
        title: t("commands.quit.title"),
        subtitle: t("commands.quit.subtitle"),
        keywords: ["quit", "exit", "close", "stop", "退出", "关闭"],
        icon: "power",
        run: () => platform.quit(),
      },
    ];
  }, [
    clearQuery,
    onOpenClipboard,
    onOpenSettings,
    paths,
    drives,
    settings.fileSearch,
    showToast,
    t,
  ]);

  const quickActions = useMemo(
    () =>
      commands
        .filter((command) => QUICK_ACTION_IDS.includes(command.id))
        .map(commandToItem),
    [commands],
  );

  const recentToItem = useCallback(
    (recent: RecentItem): LauncherItem | null => {
      if (recent.kind === "command" && recent.id.startsWith("custom:")) {
        const command = customCommands.find(
          (entry) => `custom:${entry.id}` === recent.id,
        );
        return command ? commandItem(command) : null;
      }

      if (recent.kind === "command") {
        const command = commands.find((entry) => entry.id === recent.id);
        return command ? commandToItem(command) : null;
      }

      if (!recent.path) return null;
      return {
        id: recent.id,
        kind: recent.kind,
        title: recent.title,
        subtitle:
          recent.kind === "app" ? t("common.application") : recent.subtitle,
        path: recent.path,
        extension: recent.extension,
        run: () => platform.openPath(recent.path!),
      };
    },
    [commandItem, commands, customCommands, t],
  );

  const groups = useMemo<ResultGroup[]>(() => {
    if (scope) {
      return [
        {
          id: "command",
          label: scope.title,
          color: "var(--group-label)",
          items: [scopedItem(scope, argument)],
        },
      ];
    }

    if (searchFilters.recentOnly) {
      const recentQuery = searchFilters.text.toLocaleLowerCase();
      const recentItems = recents
        .filter(
          (recent) =>
            !recentQuery ||
            recent.title.toLocaleLowerCase().includes(recentQuery),
        )
        .map(recentToItem)
        .filter((item): item is LauncherItem => Boolean(item));
      return recentItems.length > 0
        ? [
            {
              id: "recent",
              label: t("recent.label"),
              color: "var(--group-label)",
              items: recentItems,
            },
          ]
        : [];
    }

    const base = buildGroups(
      allApps,
      fileResults,
      commands,
      searchQuery,
      runPath,
      {
        app: t("groups.applications"),
        command: t("groups.commands"),
        file: t("groups.files"),
        applicationSubtitle: t("common.application"),
      },
      searchFilters,
    );

    const canShowCommands =
      !searchFilters.appOnly &&
      !searchFilters.fileOnly &&
      !searchFilters.recentOnly;
    const customItems = canShowCommands
      ? customCommands
          .map((command) => ({
            command,
            score: command.enabled
              ? scoreCustomCommand(command, searchFilters.text)
              : 0,
          }))
          .filter((entry) => entry.score > 0)
          .sort((a, b) => b.score - a.score)
          .slice(0, 3)
          .map((entry) => commandItem(entry.command))
      : [];

    // Commands are actions, so they come before apps and files. Custom
    // commands sit ahead of the built-in ones.
    const commandBase = base.find((group) => group.id === "command");
    const commandItems = commandBase?.items ?? [];
    const driveItems = commandItems.filter((item) =>
      item.id.startsWith("drive:"),
    );
    const otherCommandItems = commandItems.filter(
      (item) => !item.id.startsWith("drive:"),
    );
    const commandGroup: ResultGroup = {
      id: "command",
      label: commandBase?.label ?? t("groups.commands"),
      color: commandBase?.color ?? "var(--group-label)",
      items: [...customItems, ...otherCommandItems],
    };
    const driveGroup: ResultGroup | null =
      driveItems.length > 0
        ? {
            id: "drive",
            label: t("groups.drives"),
            color: "var(--group-label)",
            items: driveItems,
          }
        : null;
    const appGroup = base.find((group) => group.id === "app");
    const fileGroup = base.find((group) => group.id === "file");

    const pluginGroup: ResultGroup | null =
      canShowCommands && pluginResults && pluginResults.items.length > 0
        ? {
            id: "plugin",
            label: pluginResults.plugin.name,
            color: "var(--group-label)",
            items: pluginResults.items.map((item) =>
              pluginToItem(pluginResults.plugin, item),
            ),
          }
        : null;

    const smartResult = hasSearchFilters(searchFilters)
      ? null
      : evaluateSmartQuery(searchFilters.text);
    const smartGroup: ResultGroup | null = smartResult
      ? {
          id: "smart",
          label: t("groups.calculator"),
          color: "var(--group-label)",
          items: [
            {
              id: `smart:${smartResult.value}`,
              kind: "utility",
              title: smartResult.title,
              subtitle: smartResult.subtitle,
              commandIcon: "calculator",
              keepOpen: true,
              run: async () => {
                await platform.copyClipboard(smartResult.copyText);
                showToast(t("toast.resultCopied"));
              },
            },
          ],
        }
      : null;

    const windowGroup: ResultGroup | null =
      canShowCommands && searchFilters.text.trim()
        ? (() => {
            const matches = searchWindows(windows, searchFilters.text);
            if (matches.length === 0) return null;
            return {
              id: "window",
              label: t("groups.windows"),
              color: "var(--group-label)",
              items: matches.map((window) => ({
                id: `window:${window.id}`,
                kind: "window" as const,
                title: window.title,
                subtitle: [
                  window.processName,
                  window.minimized ? t("windows.minimized") : "",
                ]
                  .filter(Boolean)
                  .join(" · "),
                windowId: window.id,
                commandIcon: "window" as const,
                run: () => platform.controlWindow(window.id, "activate"),
              })),
            };
          })()
        : null;

    return [
      smartGroup,
      driveGroup,
      pluginGroup,
      commandGroup,
      appGroup,
      windowGroup,
      fileGroup,
    ].filter(
      (group): group is ResultGroup => Boolean(group && group.items.length > 0),
    );
  }, [
    allApps,
    argument,
    commandItem,
    commands,
    customCommands,
    fileResults,
    pluginResults,
    pluginToItem,
    recents,
    recentToItem,
    searchQuery,
    searchFilters,
    runPath,
    scope,
    scopedItem,
    showToast,
    t,
    windows,
  ]);

  const items = useMemo(() => groups.flatMap((group) => group.items), [groups]);

  const startItems = useMemo<LauncherItem[]>(() => {
    if (!settings.showRecent || recents.length === 0) return quickActions;

    return recents
      .map(recentToItem)
      .filter((item): item is LauncherItem => Boolean(item));
  }, [
    quickActions,
    recents,
    recentToItem,
    settings.showRecent,
  ]);

  const navigateItems = useMemo(
    () => (query.trim() ? items : startItems),
    [items, query, startItems],
  );
  const selectedItem = navigateItems[selectedIndex] ?? null;

  useEffect(() => {
    let mounted = true;
    const unlisteners: UnlistenFn[] = [];

    if (!settings.fileSearch) {
      setIndexing(false);
      setFileCount(null);
      setFileResults([]);
    }

    void platform
      .listApps()
      .then((nextApps) => {
        if (mounted) setApps(nextApps);
      })
      .catch(() => {
        if (mounted) showToast(t("toast.appsLoadFailed"));
      });

    void platform
      .getQuickPaths()
      .then((nextPaths) => {
        if (mounted) setPaths(nextPaths);
      })
      .catch(() => {
        // Folder commands fall back to a toast until paths are available.
      });

    void platform
      .onFilesIndexed((count) => {
        if (!mounted || !settings.fileSearch) return;
        setFileCount(count);
        setIndexing(false);
      })
      .then((unlisten) => {
        if (mounted) unlisteners.push(unlisten);
        else unlisten();

        if (mounted && settings.fileSearch) {
          setIndexing(true);
          void platform
            .startFileIndex()
            .then((_started) => {
              if (mounted) setIndexing(true);
            })
            .catch(() => {
              if (mounted) setIndexing(false);
            });
        }
      });

    void platform
      .onLauncherShown(() => {
        if (!mounted) return;
        setShowNonce((value) => value + 1);
      })
      .then((unlisten) => {
        if (mounted) unlisteners.push(unlisten);
        else unlisten();
      });

    void platform
      .onLauncherHidden(() => {
        if (!mounted) return;
        clearQuery();
      })
      .then((unlisten) => {
        if (mounted) unlisteners.push(unlisten);
        else unlisten();
      });

    return () => {
      mounted = false;
      unlisteners.forEach((unlisten) => unlisten());
      if (toastTimer.current) {
        window.clearTimeout(toastTimer.current);
      }
    };
  }, [clearQuery, settings.fileSearch, showToast, t]);

  // Drives can change while the launcher is hidden (USB sticks, mapped drives),
  // so refresh them every time the window is shown.
  useEffect(() => {
    let mounted = true;

    void platform
      .listDrives()
      .then((next) => {
        if (mounted) setDrives(next);
      })
      .catch(() => {
        // Drive search is optional; keep the last known list on failure.
      });

    return () => {
      mounted = false;
    };
  }, [showNonce]);

  useEffect(() => {
    let mounted = true;
    void platform
      .listWindows()
      .then((next) => {
        if (mounted) setWindows(next);
      })
      .catch(() => {
        // Window search is optional and must not block normal launcher use.
      });
    return () => {
      mounted = false;
    };
  }, [showNonce]);

  useEffect(() => {
    const trimmed = searchFilters.text.trim();
    const requestId = ++searchRequest.current;

    // Inside a command scope the text is the command argument, not a file query.
    if (
      !settings.fileSearch ||
      !trimmed ||
      scope ||
      searchSuspended ||
      searchFilters.appOnly ||
      searchFilters.recentOnly
    ) {
      setFileResults([]);
      return;
    }

    const timer = window.setTimeout(() => {
      void platform
        .searchFiles(
          trimmed,
          searchFilters.type || searchFilters.folder ? 30 : 8,
        )
        .then((files) => {
          if (searchRequest.current === requestId) {
            setFileResults(files);
          }
        })
        .catch(() => {
          if (searchRequest.current === requestId) {
            setFileResults([]);
          }
        });
    }, FILE_SEARCH_DEBOUNCE_MS);

    return () => window.clearTimeout(timer);
  }, [scope, searchFilters, searchSuspended, settings.fileSearch]);

  useEffect(() => {
    const onStorage = (event: StorageEvent) => {
      if (event.key === RECENTS_KEY) {
        setRecents(loadRecents());
      }
    };
    window.addEventListener("storage", onStorage);
    return () => window.removeEventListener("storage", onStorage);
  }, []);

  useEffect(() => {
    setRecents((current) => {
      if (current.length <= settings.maxRecent) return current;
      const next = current.slice(0, settings.maxRecent);
      saveRecents(next);
      return next;
    });
  }, [settings.maxRecent]);

  useEffect(() => {
    const paths = new Set<string>();

    for (const group of groups) {
      for (const item of group.items) {
        if ((item.kind === "app" || item.kind === "file") && item.path) {
          paths.add(item.path);
        }
      }
    }

    for (const recent of recents) {
      if ((recent.kind === "app" || recent.kind === "file") && recent.path) {
        paths.add(recent.path);
      }
    }

    const missing = Array.from(paths).filter(
      (path) => !requestedIcons.current.has(path),
    );
    if (missing.length === 0) return;

    missing.forEach((path) => requestedIcons.current.add(path));

    void platform
      .getIcons(missing)
      .then((payloads) => {
        setIcons((current) => {
          const next = { ...current };
          for (const payload of payloads) {
            if (payload.icon) next[payload.path] = payload.icon;
          }
          return next;
        });
      })
      .catch(() => {
        missing.forEach((path) => requestedIcons.current.delete(path));
      });
  }, [groups, recents]);

  useEffect(() => {
    setSelectedIndex((current) => {
      if (navigateItems.length === 0) return 0;
      return Math.min(current, navigateItems.length - 1);
    });
  }, [navigateItems.length]);

  const moveSelection = useCallback(
    (delta: number) => {
      if (navigateItems.length === 0) return;
      setSelectedIndex((current) => {
        const next = current + delta;
        if (next < 0) return 0;
        if (next >= navigateItems.length) return navigateItems.length - 1;
        return next;
      });
    },
    [navigateItems.length],
  );

  const rememberRecent = useCallback((item: LauncherItem) => {
    if (item.kind === "command" && item.id === "quit") return;
    if (item.kind !== "app" && item.kind !== "file" && item.kind !== "command") {
      return;
    }
    // Plugin results are ephemeral (they depend on the query), so they never
    // enter the recent list.
    if (item.id.startsWith("plugin:")) return;

    setRecents((current) => {
      const next: RecentItem[] = [
        {
          id: item.id,
          kind: item.kind,
          title: item.title,
          subtitle: item.subtitle,
          path: item.path,
          extension: item.extension,
          commandIcon: item.commandIcon,
          at: Date.now(),
        },
        ...current.filter((recent) => recent.id !== item.id),
      ].slice(0, settings.maxRecent);

      saveRecents(next);
      return next;
    });
  }, [settings.maxRecent]);

  const removeRecent = useCallback((id: string) => {
    setRecents((current) => {
      const next = current.filter((recent) => recent.id !== id);
      saveRecents(next);
      return next;
    });
  }, []);

  const clearRecents = useCallback(() => {
    setRecents([]);
    saveRecents([]);
  }, []);

  const confirmSelection = useCallback(async () => {
    const item = navigateItems[selectedIndex];
    if (!item) return;

    rememberRecent(item);

    try {
      await item.run();
      if (!item.keepOpen) {
        hide();
      }
    } catch {
      showToast(t("toast.actionFailed"));
    }
  }, [hide, navigateItems, rememberRecent, selectedIndex, showToast, t]);

  const execute = useCallback(
    async (item: LauncherItem) => {
      rememberRecent(item);

      try {
        await item.run();
        if (!item.keepOpen) {
          hide();
        }
      } catch {
        showToast(t("toast.actionFailed"));
      }
    },
    [hide, rememberRecent, showToast, t],
  );

  return {
    apps: allApps,
    paths,
    query,
    scope,
    argument,
    activateScope,
    setQuery,
    groups,
    items,
    navigateItems,
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
    showToast,
    toast,
    indexing,
    fileCount,
    showNonce,
  };
}
