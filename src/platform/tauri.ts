import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

import type {
  AppEntry,
  FileEntry,
  IconPayload,
  QuickPaths,
} from "../core/types";

export interface BrowserInfo {
  id: string;
  name: string;
  exe: string;
}

export type DriveKind = "fixed" | "removable" | "remote" | "ramdisk";

export interface DriveInfo {
  letter: string;
  path: string;
  kind: DriveKind;
}

export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  entry: string;
  args: string[];
  keywords: string[];
  permissions: string[];
  timeoutMs: number;
}

export interface PluginIcon {
  type: string;
  value: string;
}

export type PluginAction =
  | { type: "openUrl"; url: string }
  | { type: "openPath"; path: string };

export interface PluginItem {
  id: string;
  title: string;
  subtitle: string;
  icon: PluginIcon | null;
  action: PluginAction | null;
}

export interface ClipboardEntry {
  id: string;
  text: string;
  createdAt: number;
  pinned: boolean;
}

export const platform = {
  listApps(): Promise<AppEntry[]> {
    return invoke<AppEntry[]>("list_apps");
  },

  getQuickPaths(): Promise<QuickPaths> {
    return invoke<QuickPaths>("get_quick_paths");
  },

  listDrives(): Promise<DriveInfo[]> {
    return invoke<DriveInfo[]>("list_drives");
  },

  startFileIndex(): Promise<boolean> {
    return invoke<boolean>("start_file_index");
  },

  searchFiles(query: string, limit = 8): Promise<FileEntry[]> {
    return invoke<FileEntry[]>("search_files", { query, limit });
  },

  getIcons(paths: string[]): Promise<IconPayload[]> {
    return invoke<IconPayload[]>("get_icons", { paths });
  },

  listBrowsers(): Promise<BrowserInfo[]> {
    return invoke<BrowserInfo[]>("list_browsers");
  },

  openUrl(
    url: string,
    browser: string,
    newWindow: boolean,
  ): Promise<void> {
    return invoke<void>("open_url", {
      url,
      browser: browser === "default" ? null : browser,
      newWindow,
    });
  },

  pickExecutable(): Promise<string | null> {
    return invoke<string | null>("pick_executable");
  },

  pickFile(): Promise<string | null> {
    return invoke<string | null>("pick_file");
  },

  pickFolder(): Promise<string | null> {
    return invoke<string | null>("pick_folder");
  },

  scanFolderApps(folder: string): Promise<AppEntry[]> {
    return invoke<AppEntry[]>("scan_folder_apps", { folder });
  },

  scanIndexFolders(folders: string[]): Promise<AppEntry[]> {
    return invoke<AppEntry[]>("scan_index_folders", { folders });
  },

  listPlugins(): Promise<PluginManifest[]> {
    return invoke<PluginManifest[]>("list_plugins");
  },

  installPlugin(source: string): Promise<PluginManifest> {
    return invoke<PluginManifest>("install_plugin", { source });
  },

  uninstallPlugin(id: string): Promise<void> {
    return invoke<void>("uninstall_plugin", { id });
  },

  stopPlugin(id: string): Promise<void> {
    return invoke<void>("stop_plugin", { id });
  },

  pluginsDir(): Promise<string> {
    return invoke<string>("plugins_dir");
  },

  searchPlugin(
    id: string,
    query: string,
    keyword: string,
  ): Promise<PluginItem[]> {
    return invoke<PluginItem[]>("search_plugin", { id, query, keyword });
  },

  listClipboard(): Promise<ClipboardEntry[]> {
    return invoke<ClipboardEntry[]>("list_clipboard");
  },

  copyClipboard(text: string): Promise<void> {
    return invoke<void>("copy_clipboard", { text });
  },

  toggleClipboardPin(id: string): Promise<void> {
    return invoke<void>("toggle_clipboard_pin", { id });
  },

  deleteClipboardItem(id: string): Promise<void> {
    return invoke<void>("delete_clipboard_item", { id });
  },

  clearClipboard(): Promise<void> {
    return invoke<void>("clear_clipboard");
  },

  setClipboardCapture(enabled: boolean): Promise<void> {
    return invoke<void>("set_clipboard_capture", { enabled });
  },

  launchProgram(program: string, args: string): Promise<void> {
    return invoke<void>("launch_program", { program, args });
  },

  openPath(path: string): Promise<void> {
    return invoke<void>("open_path", { path });
  },

  hide(): Promise<void> {
    return getCurrentWindow().hide();
  },

  closeCurrentWindow(): Promise<void> {
    return getCurrentWindow().close();
  },

  openSettingsWindow(): Promise<void> {
    return invoke<void>("open_settings_window");
  },

  startDragging(): Promise<void> {
    return invoke<void>("start_window_drag");
  },

  quit(): Promise<void> {
    return invoke<void>("quit_app");
  },

  setGlobalShortcut(shortcut: string): Promise<void> {
    return invoke<void>("set_global_shortcut", { shortcut });
  },

  getShortcutStatus(): Promise<{
    current: string | null;
    error: string | null;
  }> {
    return invoke("get_shortcut_status");
  },

  setShortcutCapture(active: boolean): Promise<void> {
    return invoke<void>("set_shortcut_capture", { active });
  },

  onShortcutCaptured(handler: (shortcut: string) => void): Promise<UnlistenFn> {
    return listen<string>("shortcut-captured", (event) =>
      handler(event.payload),
    );
  },

  getLaunchAtStartup(): Promise<boolean> {
    return invoke<boolean>("get_launch_at_startup");
  },

  setLaunchAtStartup(enabled: boolean): Promise<void> {
    return invoke<void>("set_launch_at_startup", { enabled });
  },

  setTrayLanguage(language: "en" | "zh"): Promise<void> {
    return invoke<void>("set_tray_language", { language });
  },

  resetWindowPosition(): Promise<void> {
    return invoke<void>("reset_window_position");
  },

  setAppTheme(theme: "light" | "dark"): Promise<void> {
    return invoke<void>("set_app_theme", { theme });
  },

  exportSettings(json: string): Promise<string | null> {
    return invoke<string | null>("export_settings", { json });
  },

  importSettings(): Promise<string | null> {
    return invoke<string | null>("import_settings");
  },

  onLauncherShown(handler: () => void): Promise<UnlistenFn> {
    return listen("launcher-shown", handler);
  },

  onLauncherHidden(handler: () => void): Promise<UnlistenFn> {
    return listen("launcher-hidden", handler);
  },

  onFilesIndexed(handler: (count: number) => void): Promise<UnlistenFn> {
    return listen<number>("files-indexed", (event) => handler(event.payload));
  },

  onClipboardUpdated(handler: () => void): Promise<UnlistenFn> {
    return listen("clipboard-updated", handler);
  },
};
