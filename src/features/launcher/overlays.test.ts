import { describe, expect, it } from "vitest";

import type { LauncherItem } from "../../core/types";
import { INITIAL_OVERLAY_STATE, overlayReducer } from "./overlays";

const app: LauncherItem = {
  id: "app",
  kind: "app",
  title: "Terminal",
  subtitle: "Application",
  path: "C:\\Apps\\Terminal.exe",
  run: () => undefined,
};

const file: LauncherItem = {
  id: "file",
  kind: "file",
  title: "notes.txt",
  subtitle: "Documents",
  path: "C:\\Users\\Ada\\Documents\\notes.txt",
  extension: "txt",
  run: () => undefined,
};

describe("overlayReducer", () => {
  it("closes action and preview overlays when the launcher hides", () => {
    const withActions = overlayReducer(INITIAL_OVERLAY_STATE, {
      type: "open-actions",
      item: app,
    });
    const cleared = overlayReducer(
      {
        ...withActions,
        previewItem: file,
      },
      { type: "close-all" },
    );

    expect(cleared).toEqual(INITIAL_OVERLAY_STATE);
  });

  it("opens one overlay at a time", () => {
    const withActions = overlayReducer(INITIAL_OVERLAY_STATE, {
      type: "open-actions",
      item: app,
    });
    const preview = overlayReducer(withActions, {
      type: "open-preview",
      item: file,
    });

    expect(preview.actionItem).toBeNull();
    expect(preview.previewItem).toBe(file);
  });

  it("clamps action keyboard navigation to available actions", () => {
    const opened = overlayReducer(INITIAL_OVERLAY_STATE, {
      type: "open-actions",
      item: file,
    });
    const down = overlayReducer(opened, {
      type: "move-actions",
      delta: 1,
      count: 5,
    });
    const end = overlayReducer(down, {
      type: "move-actions",
      delta: 20,
      count: 5,
    });

    expect(down.actionIndex).toBe(1);
    expect(end.actionIndex).toBe(4);
  });
});
