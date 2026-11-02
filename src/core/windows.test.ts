import { describe, expect, it } from "vitest";

import type { WindowEntry } from "./types";
import { searchWindows } from "./windows";

const windows: WindowEntry[] = [
  {
    id: "1",
    title: "NEXORA source code - Visual Studio Code",
    processName: "Code.exe",
    processPath: "C:\\Apps\\Code.exe",
    minimized: false,
  },
  {
    id: "2",
    title: "Project notes - Notepad",
    processName: "notepad.exe",
    processPath: "C:\\Windows\\notepad.exe",
    minimized: true,
  },
];

describe("searchWindows", () => {
  it("matches title and process names case-insensitively", () => {
    expect(searchWindows(windows, "CODE")[0]?.id).toBe("1");
    expect(searchWindows(windows, "notepad")[0]?.id).toBe("2");
  });

  it("returns no windows for an empty query", () => {
    expect(searchWindows(windows, " ")).toEqual([]);
  });
});
