import { describe, expect, it } from "vitest";

import { hasReachedDragThreshold } from "./drag";

describe("launcher drag threshold", () => {
  it("keeps normal clicks and text selection inside the search box", () => {
    expect(hasReachedDragThreshold(0, 0)).toBe(false);
    expect(hasReachedDragThreshold(3, 2)).toBe(false);
  });

  it("starts dragging after deliberate pointer movement", () => {
    expect(hasReachedDragThreshold(4, 0)).toBe(true);
    expect(hasReachedDragThreshold(-3, -3)).toBe(true);
  });
});
