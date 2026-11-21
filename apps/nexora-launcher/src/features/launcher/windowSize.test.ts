import { describe, expect, it } from "vitest";

import {
  LAUNCHER_WINDOW_HEIGHT,
  launcherWindowWidth,
} from "./windowSize";

describe("launcher window sizing", () => {
  it("keeps the search panel centered inside the native window", () => {
    expect(launcherWindowWidth(500)).toBe(580);
    expect(launcherWindowWidth(650)).toBe(730);
    expect(launcherWindowWidth(800)).toBe(880);
    expect(LAUNCHER_WINDOW_HEIGHT).toBe(620);
  });
});
