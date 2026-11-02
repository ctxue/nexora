import type { LauncherItem } from "./types";

function isPreviewable(item: LauncherItem | undefined): boolean {
  return Boolean(item?.kind === "file" && item.path);
}

/**
 * Finds the next previewable file while preserving the current file at either
 * edge. Keeping the current selection is less surprising than wrapping around.
 */
export function findPreviewIndex(
  items: LauncherItem[],
  currentIndex: number,
  delta: number,
): number | null {
  if (items.length === 0 || delta === 0) return null;

  const current = Math.max(0, Math.min(items.length - 1, currentIndex));
  for (
    let index = current + delta;
    index >= 0 && index < items.length;
    index += delta
  ) {
    if (isPreviewable(items[index])) return index;
  }

  return isPreviewable(items[current]) ? current : null;
}
