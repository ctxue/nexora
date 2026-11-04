import { describe, expect, it } from "vitest";

import type { LauncherItem } from "./types";
import { findPreviewIndex } from "./navigation";

const items: LauncherItem[] = [
  {
    id: "app",
    kind: "app",
    title: "Terminal",
    subtitle: "Application",
    path: "C:\\Terminal.exe",
    run: () => undefined,
  },
  {
    id: "text",
    kind: "file",
    title: "notes.txt",
    subtitle: "Documents",
    path: "C:\\notes.txt",
    extension: "txt",
    run: () => undefined,
  },
  {
    id: "command",
    kind: "command",
    title: "Settings",
    subtitle: "Command",
    run: () => undefined,
  },
  {
    id: "image",
    kind: "file",
    title: "photo.png",
    subtitle: "Pictures",
    path: "C:\\photo.png",
    extension: "png",
    run: () => undefined,
  },
];

describe("findPreviewIndex", () => {
  it("skips non-previewable results while moving forward", () => {
    expect(findPreviewIndex(items, 0, 1)).toBe(1);
    expect(findPreviewIndex(items, 1, 1)).toBe(3);
  });

  it("skips non-previewable results while moving backward", () => {
    expect(findPreviewIndex(items, 3, -1)).toBe(1);
  });

  it("stays on the current file at the end of the list", () => {
    expect(findPreviewIndex(items, 3, 1)).toBe(3);
  });
});
