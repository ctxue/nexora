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
  | "terminal";

export interface LauncherCommand {
  id: string;
  title: string;
  subtitle: string;
  keywords: string[];
  icon: CommandIcon;
  run: () => void | Promise<void>;
}

export type ResultKind = "app" | "file" | "command";

export interface LauncherItem {
  id: string;
  kind: ResultKind;
  title: string;
  subtitle: string;
  path?: string;
  extension?: string;
  commandIcon?: CommandIcon;
  keepOpen?: boolean;
  run: () => void | Promise<void>;
}

export interface ResultGroup {
  /** Group key; built-ins use "app" / "command" / "file", plugins add their own. */
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
