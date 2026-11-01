import { describe, expect, it } from "vitest";

import { searchApps, searchCommands } from "./search";
import type { AppEntry, LauncherCommand } from "./types";

const apps: AppEntry[] = [
  {
    id: "vscode",
    name: "Visual Studio Code",
    path: "C:\\Apps\\Code.exe",
    source: "Start Menu",
  },
  {
    id: "chrome",
    name: "Google Chrome",
    path: "C:\\Apps\\chrome.exe",
    source: "Start Menu",
  },
  {
    id: "wechat",
    name: "微信",
    path: "C:\\Apps\\WeChat.exe",
    source: "Start Menu",
  },
];

describe("searchApps", () => {
  it("matches initials across application words", () => {
    expect(searchApps(apps, "vsc")[0]?.name).toBe("Visual Studio Code");
  });

  it("matches Chinese pinyin initials", () => {
    expect(searchApps(apps, "wx")[0]?.name).toBe("微信");
  });

  it("ranks an exact application name first", () => {
    const exact = { ...apps[1], id: "chrome-exact", name: "Chrome" };

    expect(searchApps([...apps, exact], "chrome")[0]?.name).toBe("Chrome");
  });
});

describe("searchCommands", () => {
  it("matches command keywords as well as titles", () => {
    const commands: LauncherCommand[] = [
      {
        id: "settings",
        title: "Open preferences",
        subtitle: "Change NEXORA",
        keywords: ["settings"],
        icon: "settings",
        run: () => undefined,
      },
    ];

    expect(searchCommands(commands, "settings")).toEqual(commands);
  });
});
