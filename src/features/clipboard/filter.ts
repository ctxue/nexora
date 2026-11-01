import type { ClipboardEntry } from "../../platform/tauri";

function normalize(value: string): string {
  return value.toLocaleLowerCase().replace(/\s+/gu, " ").trim();
}

export function filterClipboardEntries(
  entries: ClipboardEntry[],
  query: string,
): ClipboardEntry[] {
  const normalizedQuery = normalize(query);

  return entries
    .filter(
      (entry) =>
        !normalizedQuery || normalize(entry.text).includes(normalizedQuery),
    )
    .sort(
      (left, right) =>
        Number(right.pinned) - Number(left.pinned) ||
        right.createdAt - left.createdAt,
    );
}

export function buildClipboardPreview(text: string, maxChars = 180): string {
  const collapsed = text.replace(/\s+/gu, " ").trim();
  const characters = Array.from(collapsed);
  if (characters.length <= maxChars) return collapsed;
  return `${characters.slice(0, Math.max(0, maxChars)).join("")}…`;
}
