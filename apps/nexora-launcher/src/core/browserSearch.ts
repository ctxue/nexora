import { scoreText } from "./search";
import type {
  BrowserEntry,
  BrowserEntryKind,
  ResultGroup,
} from "./types";

interface BrowserSearchLabels {
  tabs: string;
  bookmarks: string;
  history: string;
}

const GROUP_ORDER: BrowserEntryKind[] = ["tab", "bookmark", "history"];

function groupLabel(kind: BrowserEntryKind, labels: BrowserSearchLabels) {
  if (kind === "tab") return labels.tabs;
  if (kind === "bookmark") return labels.bookmarks;
  return labels.history;
}

export function searchBrowserGroups(
  entries: BrowserEntry[],
  query: string,
  labels: BrowserSearchLabels,
  openEntry: (entry: BrowserEntry) => void | Promise<void>,
): ResultGroup[] {
  const normalized = query.toLocaleLowerCase().trim();
  if (!normalized) return [];

  return GROUP_ORDER.map((kind) => {
    const items = entries
      .filter((entry) => entry.kind === kind)
      .map((entry) => ({
        entry,
        score: scoreText(normalized, entry.title, entry.url),
      }))
      .filter((candidate) => candidate.score > 0)
      .sort(
        (a, b) =>
          b.score - a.score ||
          (b.entry.lastVisited ?? 0) - (a.entry.lastVisited ?? 0) ||
          a.entry.title.localeCompare(b.entry.title),
      )
      .slice(0, 5)
      .map(({ entry }) => ({
        id: entry.id,
        kind: entry.kind,
        title: entry.title || entry.url,
        subtitle: `${entry.browserName} · ${entry.url}`,
        url: entry.url,
        browserExe: entry.browserExe,
        browserTabId: entry.tabId,
        commandIcon: entry.kind === "tab" ? ("window" as const) : ("link" as const),
        run: () => openEntry(entry),
      }));

    return {
      id: kind,
      label: groupLabel(kind, labels),
      color: "var(--group-label)",
      items,
    };
  }).filter((group) => group.items.length > 0);
}
