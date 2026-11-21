export interface AppEntry {
  id: string;
  name: string;
  path: string;
  source: string;
}

export interface FileEntry {
  id: string;
  name: string;
  path: string;
  extension: string;
  parent: string;
  isDirectory: boolean;
}

export interface WindowEntry {
  id: string;
  title: string;
  processName: string;
  processPath: string | null;
  minimized: boolean;
}

export type BrowserEntryKind = "bookmark" | "history" | "tab";

export interface BrowserEntry {
  id: string;
  kind: BrowserEntryKind;
  title: string;
  url: string;
  browserName: string;
  browserExe: string | null;
  lastVisited?: number;
  tabId?: number;
  windowId?: number;
}

export interface QuickPaths {
  home: string;
  desktop: string;
  documents: string;
  downloads: string;
}

export type CommandIcon =
  | "downloads"
  | "desktop"
  | "documents"
  | "computer"
  | "trash"
  | "clipboard"
  | "settings"
  | "refresh"
  | "power"
  | "globe"
  | "search"
  | "link"
  | "folder"
  | "terminal"
  | "calculator"
  | "window";

export interface LauncherCommand {
  id: string;
  title: string;
  subtitle: string;
  keywords: string[];
  icon: CommandIcon;
  run: () => void | Promise<void>;
}

export type ResultKind =
  | "app"
  | "file"
  | "command"
  | "utility"
  | "window"
  | "bookmark"
  | "history"
  | "tab";

export interface LauncherItem {
  id: string;
  kind: ResultKind;
  title: string;
  subtitle: string;
  path?: string;
  extension?: string;
  isDirectory?: boolean;
  url?: string;
  browserExe?: string | null;
  browserTabId?: number;
  windowId?: string;
  commandIcon?: CommandIcon;
  keepOpen?: boolean;
  run: () => void | Promise<void>;
}

export interface ResultGroup {
  id: string;
  label: string;
  color: string;
  items: LauncherItem[];
}

export interface RecentItem {
  id: string;
  kind: ResultKind;
  title: string;
  subtitle: string;
  path?: string;
  extension?: string;
  commandIcon?: CommandIcon;
  at: number;
}

export interface IconPayload {
  path: string;
  icon: string | null;
}
