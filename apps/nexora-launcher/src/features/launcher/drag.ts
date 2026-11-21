const LAUNCHER_DRAG_THRESHOLD = 4;

export function hasReachedDragThreshold(
  deltaX: number,
  deltaY: number,
  threshold = LAUNCHER_DRAG_THRESHOLD,
): boolean {
  return deltaX * deltaX + deltaY * deltaY >= threshold * threshold;
}
