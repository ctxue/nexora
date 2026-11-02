import type { CommandIcon } from "./types";

/** What a custom command does when it runs. */
export type CommandKind = "web-search" | "open-url" | "open-path" | "launch";

/** The payload of a command. Unused fields stay empty so storage stays flat. */
export interface CommandAction {
  kind: CommandKind;
  /** `web-search`: URL template containing `{query}`. */
  template: string;
  /** `open-url` URL, `open-path` path, or `launch` program. */
  target: string;
  /** `launch`: raw argument template, may contain `{query}`. */
  args: string;
  /** Browser executable, or `"default"`. */
  browser: string;
  newWindow: boolean;
}

export interface CustomCommand {
  id: string;
  /** Trigger word typed in the launcher, lower case, no spaces. */
  keyword: string;
  title: string;
  subtitle: string;
  icon: CommandIcon;
  action: CommandAction;
  enabled: boolean;
  createdAt: number;
  updatedAt: number;
}

export const COMMAND_LIMIT = 100;

/** 1–16 characters, starts with a letter or digit. */
export const KEYWORD_PATTERN = /^[a-z0-9][a-z0-9_-]{0,15}$/i;

export const COMMAND_ICON_CHOICES: CommandIcon[] = [
  "globe",
  "search",
  "link",
  "folder",
  "terminal",
  "documents",
  "downloads",
  "settings",
  "refresh",
  "power",
];

const COMMAND_KINDS: CommandKind[] = [
  "web-search",
  "open-url",
  "open-path",
  "launch",
];

export type CommandError =
  | "title"
  | "keywordFormat"
  | "keywordTaken"
  | "template"
  | "templateQuery"
  | "url"
  | "target"
  | null;

function isHttpUrl(value: string): boolean {
  const trimmed = value.trim().toLowerCase();
  return trimmed.startsWith("http://") || trimmed.startsWith("https://");
}

/** Whether the command expects text after its keyword (`edge <query>`). */
export function acceptsArgument(command: CustomCommand): boolean {
  if (command.action.kind === "web-search") return true;
  if (command.action.kind === "launch") return command.action.args.includes("{query}");
  return false;
}

/** Finds an enabled argument-taking command by its exact keyword. */
export function findArgumentCommand(
  keyword: string,
  commands: CustomCommand[],
): CustomCommand | null {
  const normalized = keyword.trim().toLowerCase();
  if (!normalized) return null;
  return (
    commands.find(
      (entry) =>
        entry.enabled &&
        acceptsArgument(entry) &&
        entry.keyword.toLowerCase() === normalized,
    ) ?? null
  );
}

/** Replaces `{query}` in a URL template, percent-encoding the argument. */
export function buildUrl(template: string, argument: string): string {
  return template.replaceAll("{query}", encodeURIComponent(argument));
}

/** Quotes one argument for the Windows command line. */
export function quoteWindowsArgument(value: string): string {
  if (value.length > 0 && !/[\s"]/u.test(value)) return value;
  // Escape runs of backslashes that precede a quote, then double trailing ones.
  const escaped = value.replace(/(\\*)"/g, '$1$1\\"').replace(/(\\+)$/, "$1$1");
  return `"${escaped}"`;
}

/** Fills a raw argument template with the (quoted) launcher argument. */
export function buildArguments(template: string, argument: string): string {
  return template.replaceAll("{query}", quoteWindowsArgument(argument));
}

function normalizeAction(value: unknown): CommandAction {
  const record =
    value && typeof value === "object"
      ? (value as Record<string, unknown>)
      : ({} as Record<string, unknown>);

  const kind = COMMAND_KINDS.includes(record.kind as CommandKind)
    ? (record.kind as CommandKind)
    : "web-search";

  return {
    kind,
    template: typeof record.template === "string" ? record.template : "",
    target: typeof record.target === "string" ? record.target : "",
    args: typeof record.args === "string" ? record.args : "",
    browser:
      typeof record.browser === "string" && record.browser
        ? record.browser
        : "default",
    newWindow: record.newWindow === true,
  };
}

/** Coerces stored or imported data into a safe command list. */
export function normalizeCommands(value: unknown): CustomCommand[] {
  if (!Array.isArray(value)) return [];

  const seen = new Set<string>();
  const commands: CustomCommand[] = [];

  for (const entry of value) {
    if (!entry || typeof entry !== "object") continue;

    const record = entry as Partial<CustomCommand>;
    const keyword =
      typeof record.keyword === "string"
        ? record.keyword.trim().toLowerCase()
        : "";

    if (!KEYWORD_PATTERN.test(keyword) || seen.has(keyword)) continue;

    const title =
      typeof record.title === "string" && record.title.trim()
        ? record.title.trim()
        : keyword;

    seen.add(keyword);
    commands.push({
      id:
        typeof record.id === "string" && record.id
          ? record.id
          : `cmd-${keyword}-${commands.length}`,
      keyword,
      title,
      subtitle: typeof record.subtitle === "string" ? record.subtitle : "",
      icon: COMMAND_ICON_CHOICES.includes(record.icon as CommandIcon)
        ? (record.icon as CommandIcon)
        : "globe",
      action: normalizeAction(record.action),
      enabled: record.enabled !== false,
      createdAt:
        typeof record.createdAt === "number" ? record.createdAt : Date.now(),
      updatedAt:
        typeof record.updatedAt === "number" ? record.updatedAt : Date.now(),
    });

    if (commands.length >= COMMAND_LIMIT) break;
  }

  return commands;
}

/** A blank command for the settings editor. */
export function createCommand(): CustomCommand {
  const now = Date.now();
  return {
    id: `cmd-${now.toString(36)}-${Math.random().toString(36).slice(2, 7)}`,
    keyword: "",
    title: "",
    subtitle: "",
    icon: "globe",
    action: {
      kind: "web-search",
      template: "",
      target: "",
      args: "",
      browser: "default",
      newWindow: false,
    },
    enabled: true,
    createdAt: now,
    updatedAt: now,
  };
}

/** Returns the first problem with a draft, or `null` when it can be saved. */
export function validateCommand(
  command: CustomCommand,
  all: CustomCommand[],
): CommandError {
  const keyword = command.keyword.trim().toLowerCase();

  if (!command.title.trim()) return "title";
  if (!KEYWORD_PATTERN.test(command.keyword.trim())) return "keywordFormat";

  const taken = all.some(
    (entry) => entry.id !== command.id && entry.keyword.toLowerCase() === keyword,
  );
  if (taken) return "keywordTaken";

  const action = command.action;
  switch (action.kind) {
    case "web-search":
      if (!isHttpUrl(action.template)) return "template";
      if (!action.template.includes("{query}")) return "templateQuery";
      return null;
    case "open-url":
      return isHttpUrl(action.target) ? null : "url";
    default:
      return action.target.trim() ? null : "target";
  }
}

export interface ParsedQuery {
  scope: CustomCommand | null;
  argument: string;
}

/**
 * Splits `edge some words` into the command that owns `edge` and the argument.
 * Only commands that accept an argument can become a scope.
 */
export function parseQuery(
  raw: string,
  commands: CustomCommand[],
): ParsedQuery {
  const match = /^(\S+)\t([\s\S]*)$/.exec(raw);
  if (!match) return { scope: null, argument: raw };

  const command = findArgumentCommand(match[1], commands);

  return command
    ? { scope: command, argument: match[2] }
    : { scope: null, argument: raw };
}

/** Ranks commands shown in normal (unscoped) search results. */
export function scoreCustomCommand(
  command: CustomCommand,
  rawQuery: string,
): number {
  const query = rawQuery.trim().toLowerCase();
  if (!query) return 0;

  const keyword = command.keyword.toLowerCase();
  const title = command.title.toLowerCase();

  if (keyword === query) return 230;
  if (keyword.startsWith(query)) return 200;
  if (title === query) return 180;
  if (title.startsWith(query)) return 170;
  if (title.includes(query)) return 140;

  const compactTitle = title.replace(/\s+/g, "");
  const compactQuery = query.replace(/\s+/g, "");
  if (compactQuery.length >= 2 && compactTitle.includes(compactQuery)) {
    return 130;
  }

  return 0;
}

/** Host shown under a command item ("cn.bing.com"). */
export function commandHost(command: CustomCommand): string {
  const candidate =
    command.action.kind === "web-search"
      ? command.action.template
      : command.action.kind === "open-url"
        ? command.action.target
        : "";

  try {
    return new URL(candidate).host;
  } catch {
    return "";
  }
}

export interface CommandPreset {
  id: string;
  title: string;
  keyword: string;
  subtitle: string;
  template: string;
  icon: CommandIcon;
}

/** Ready-made searches offered in the settings page. */
export const COMMAND_PRESETS: CommandPreset[] = [
  {
    id: "bing",
    title: "Bing",
    keyword: "bing",
    subtitle: "cn.bing.com",
    template: "https://cn.bing.com/search?q={query}",
    icon: "globe",
  },
  {
    id: "google",
    title: "Google",
    keyword: "g",
    subtitle: "google.com",
    template: "https://www.google.com/search?q={query}",
    icon: "search",
  },
  {
    id: "github",
    title: "GitHub",
    keyword: "gh",
    subtitle: "github.com",
    template: "https://github.com/search?q={query}&type=repositories",
    icon: "link",
  },
  {
    id: "bilibili",
    title: "哔哩哔哩",
    keyword: "bili",
    subtitle: "bilibili.com",
    template: "https://search.bilibili.com/all?keyword={query}",
    icon: "search",
  },
  {
    id: "zhihu",
    title: "知乎",
    keyword: "zhihu",
    subtitle: "zhihu.com",
    template: "https://www.zhihu.com/search?q={query}",
    icon: "search",
  },
];

/** Builds a command from a preset, using the default browser. */
export function commandFromPreset(preset: CommandPreset): CustomCommand {
  const now = Date.now();
  return {
    id: `cmd-${now.toString(36)}-${Math.random().toString(36).slice(2, 7)}`,
    keyword: preset.keyword,
    title: preset.title,
    subtitle: preset.subtitle,
    icon: preset.icon,
    action: {
      kind: "web-search",
      template: preset.template,
      target: "",
      args: "",
      browser: "default",
      newWindow: false,
    },
    enabled: true,
    createdAt: now,
    updatedAt: now,
  };
}
