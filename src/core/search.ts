import { pinyin } from "pinyin-pro";

import type {
  AppEntry,
  LauncherCommand,
  LauncherItem,
  ResultGroup,
} from "./types";
import { matchesFileFilters, type SearchQuery } from "./query";

const GROUP_META = {
  app: { label: "Applications", color: "var(--group-label)" },
  file: { label: "Files", color: "var(--group-label)" },
  command: { label: "Commands", color: "var(--group-label)" },
} as const;

interface PinyinFields {
  full: string;
  initials: string;
}

const PINYIN_CACHE = new Map<string, PinyinFields>();

function normalize(value: string): string {
  return value.toLocaleLowerCase().trim();
}

function compact(value: string): string {
  return normalize(value).replace(/\s+/g, "");
}

function getPinyin(text: string): PinyinFields {
  const cached = PINYIN_CACHE.get(text);
  if (cached) return cached;

  const full = pinyin(text, {
    toneType: "none",
    type: "array",
  })
    .join("")
    .toLocaleLowerCase()
    .replace(/\s+/g, "");
  const initials = pinyin(text, {
    pattern: "first",
    toneType: "none",
    type: "array",
  })
    .join("")
    .toLocaleLowerCase()
    .replace(/\s+/g, "");

  const fields = { full, initials };
  PINYIN_CACHE.set(text, fields);
  return fields;
}

function wordInitials(value: string): string {
  return normalize(value)
    .split(/[^\p{L}\p{N}]+/u)
    .filter(Boolean)
    .map((word) => word[0])
    .join("");
}

function isSubsequence(query: string, value: string): boolean {
  if (query.length < 3) return false;

  let index = 0;
  for (const character of value) {
    if (character === query[index]) {
      index += 1;
      if (index === query.length) return true;
    }
  }
  return false;
}

/**
 * A deliberately small fuzzy scorer. It handles exact/prefix/contains,
 * word initials (vsc -> Visual Studio Code), pinyin (wx -> 微信), and
 * ordered subsequences for typo-tolerant matches.
 */
function scoreField(query: string, value: string, allowFuzzy = true): number {
  const field = normalize(value);
  if (!field) return 0;

  if (field === query) return 210;
  if (field.startsWith(query)) return 185;
  if (field.includes(query)) return 155;

  const compactQuery = compact(query);
  const compactField = compact(field);

  if (compactField === compactQuery) return 205;
  if (compactField.startsWith(compactQuery)) return 180;
  if (compactField.includes(compactQuery)) return 150;

  const tokens = query.split(/\s+/).filter(Boolean);
  if (tokens.length > 1 && tokens.every((token) => field.includes(token))) {
    return 125;
  }

  if (!allowFuzzy) return 0;

  const initials = wordInitials(field);
  if (initials.startsWith(compactQuery)) return 170;
  if (initials.includes(compactQuery)) return 125;

  if (isSubsequence(compactQuery, compactField)) {
    const gaps = Math.max(0, compactField.length - compactQuery.length);
    return Math.max(45, 105 - gaps);
  }

  return 0;
}

function scoreText(query: string, text: string, extra = ""): number {
  let best = scoreField(query, text);

  if (extra) {
    best = Math.max(best, scoreField(query, extra, false) - 15);
  }

  if (/[\u3400-\u9fff]/u.test(text)) {
    const pinyinFields = getPinyin(text);
    best = Math.max(
      best,
      scoreField(query, pinyinFields.full) - 5,
      scoreField(query, pinyinFields.initials),
    );
  }

  return best;
}

export function searchApps(apps: AppEntry[], query: string): AppEntry[] {
  const normalized = normalize(query);
  if (!normalized) return [];

  return apps
    .map((app) => ({
      app,
      score: scoreText(normalized, app.name, app.path),
    }))
    .filter((entry) => entry.score > 0)
    .sort(
      (a, b) =>
        b.score - a.score ||
        a.app.name.length - b.app.name.length ||
        a.app.name.localeCompare(b.app.name),
    )
    .slice(0, 5)
    .map((entry) => entry.app);
}

export function searchCommands(
  commands: LauncherCommand[],
  query: string,
): LauncherCommand[] {
  const normalized = normalize(query);
  if (!normalized) return [];

  return commands
    .map((command) => ({
      command,
      score: scoreText(
        normalized,
        command.title,
        `${command.subtitle} ${command.keywords.join(" ")}`,
      ),
    }))
    .filter((entry) => entry.score > 0)
    .sort(
      (a, b) =>
        b.score - a.score ||
        a.command.title.length - b.command.title.length ||
        a.command.title.localeCompare(b.command.title),
    )
    .slice(0, 3)
    .map((entry) => entry.command);
}

export function commandToItem(command: LauncherCommand): LauncherItem {
  return {
    id: command.id,
    kind: "command",
    title: command.title,
    subtitle: command.subtitle,
    commandIcon: command.icon,
    keepOpen:
      command.id === "settings" ||
      command.id === "clipboard" ||
      command.id === "refresh",
    run: command.run,
  };
}

export function buildGroups(
  apps: AppEntry[],
  files: Array<{
    id: string;
    name: string;
    path: string;
    extension: string;
    parent: string;
  }>,
  commands: LauncherCommand[],
  query: string,
  runPath: (path: string) => void | Promise<void>,
  labels?: {
    app: string;
    command: string;
    file: string;
    applicationSubtitle: string;
  },
  filters?: SearchQuery,
): ResultGroup[] {
  const searchText = filters?.text ?? query;
  const appItems: LauncherItem[] = searchApps(apps, searchText).map((app) => ({
    id: app.id,
    kind: "app",
    title: app.name,
    subtitle: labels?.applicationSubtitle ?? "Application",
    path: app.path,
    run: () => runPath(app.path),
  }));

  const fileItems: LauncherItem[] = files
    .filter(
      (file) => !filters || matchesFileFilters(file, filters),
    )
    .slice(0, 5)
    .map((file) => ({
      id: file.id,
      kind: "file",
      title: file.name,
      subtitle: file.parent,
      path: file.path,
      extension: file.extension,
      run: () => runPath(file.path),
    }));

  const commandItems: LauncherItem[] = searchCommands(
    commands,
    searchText,
  ).map(commandToItem);

  const groups: ResultGroup[] = [
    {
      id: "app",
      ...GROUP_META.app,
      label: labels?.app ?? GROUP_META.app.label,
      items: appItems,
    },
    {
      id: "command",
      ...GROUP_META.command,
      label: labels?.command ?? GROUP_META.command.label,
      items: commandItems,
    },
    {
      id: "file",
      ...GROUP_META.file,
      label: labels?.file ?? GROUP_META.file.label,
      items: fileItems,
    },
  ];

  return groups.filter((group) => {
    if (group.items.length === 0) return false;
    if (filters?.appOnly) return group.id === "app";
    if (filters?.fileOnly) return group.id === "file";
    return true;
  });
}
