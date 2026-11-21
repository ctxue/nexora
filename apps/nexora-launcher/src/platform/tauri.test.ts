import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  center: vi.fn<() => Promise<void>>(),
  setSize: vi.fn<() => Promise<void>>(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    center: mocks.center,
    setSize: mocks.setSize,
  }),
  LogicalSize: class LogicalSize {
    constructor(
      public width: number,
      public height: number,
    ) {}
  },
}));

import { platform } from "./tauri";

describe("launcher window sizing", () => {
  beforeEach(() => {
    mocks.center.mockReset().mockResolvedValue(undefined);
    mocks.setSize.mockReset().mockResolvedValue(undefined);
  });

  it("recenters the launcher after changing its width", async () => {
    await platform.setLauncherWindowWidth(800, 620);

    expect(mocks.setSize).toHaveBeenCalledOnce();
    expect(mocks.center).toHaveBeenCalledOnce();
    expect(mocks.setSize.mock.invocationCallOrder[0]).toBeLessThan(
      mocks.center.mock.invocationCallOrder[0]!,
    );
  });
});
