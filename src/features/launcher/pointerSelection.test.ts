import { describe, expect, it } from "vitest";

import { shouldSelectFromPointer } from "./pointerSelection";

describe("shouldSelectFromPointer", () => {
  it("ignores mouse events that did not move from the last pointer position", () => {
    expect(
      shouldSelectFromPointer("mouse", 120, 80, { x: 120, y: 80 }),
    ).toBe(false);
  });

  it("selects after the mouse actually moves", () => {
    expect(
      shouldSelectFromPointer("mouse", 121, 80, { x: 120, y: 80 }),
    ).toBe(true);
  });

  it("does not let non-mouse pointers replace keyboard selection", () => {
    expect(shouldSelectFromPointer("touch", 200, 100, null)).toBe(false);
  });
});
