import type { WindowEntry } from "./types";

/** Lightweight local search keeps window enumeration and scoring independent. */
export function searchWindows(
  windows: WindowEntry[],
  rawQuery: string,
  limit = 5,
): WindowEntry[] {
  const query = rawQuery.trim().toLocaleLowerCase();
  if (!query) return [];

  return windows
    .map((window) => {
      const title = window.title.toLocaleLowerCase();
      const process = window.processName.toLocaleLowerCase();
      const score = title === query
        ? 500
        : title.startsWith(query)
          ? 400
          : title.includes(query)
            ? 300
            : process.includes(query)
              ? 200
              : 0;
      return { window, score };
    })
    .filter((entry) => entry.score > 0)
    .sort(
      (a, b) =>
        b.score - a.score ||
        a.window.title.length - b.window.title.length ||
        a.window.title.localeCompare(b.window.title),
    )
    .slice(0, limit)
    .map((entry) => entry.window);
}
