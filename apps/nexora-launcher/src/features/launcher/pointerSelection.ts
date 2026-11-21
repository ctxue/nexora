export interface PointerPosition {
  x: number;
  y: number;
}

export function shouldSelectFromPointer(
  pointerType: string,
  x: number,
  y: number,
  previous: PointerPosition | null,
): boolean {
  if (pointerType !== "mouse") return false;
  if (!previous) return true;
  return previous.x !== x || previous.y !== y;
}
