import { describe, expect, it } from "vitest";

import { buildGroups, searchApps, searchCommands } from "./search";
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

describe("buildGroups", () => {
  const files = [
    {
      id: "pdf",
      name: "Quarterly report.pdf",
      path: "C:\\Users\\Ada\\Downloads\\Quarterly report.pdf",
      extension: "pdf",
      parent: "Downloads",
    },
    {
      id: "text",
      name: "Quarterly notes.txt",
      path: "C:\\Users\\Ada\\Documents\\Quarterly notes.txt",
      extension: "txt",
      parent: "Documents",
    },
  ];

  it("returns only applications for an app filter", () => {
    const groups = buildGroups(
      apps,
      files,
      [],
      "code",
      () => undefined,
      undefined,
      {
        text: "code",
        appOnly: true,
        fileOnly: false,
        recentOnly: false,
        type: null,
        folder: null,
        pathRoot: null,
      },
    );

    expect(groups.map((group) => group.id)).toEqual(["app"]);
    expect(groups[0].items[0].title).toBe("Visual Studio Code");
  });

  it("applies file type and folder filters to file results", () => {
    const groups = buildGroups(
      apps,
      files,
      [],
      "quarterly",
      () => undefined,
      undefined,
      {
        text: "quarterly",
        appOnly: false,
        fileOnly: true,
        recentOnly: false,
        type: "pdf",
        folder: "downloads",
        pathRoot: null,
      },
    );

    expect(groups.map((group) => group.id)).toEqual(["file"]);
    expect(groups[0].items.map((item) => item.title)).toEqual([
      "Quarterly report.pdf",
    ]);
  });

  it("returns drive-scoped files when the query text is empty", () => {
    const driveFiles = [
      ...files,
      {
        id: "drive-project",
        name: "Project notes.md",
        path: "D:\\Projects\\Project notes.md",
        extension: "md",
        parent: "D:\\Projects",
      },
      {
        id: "drive-folder",
        name: "Projects",
        path: "D:\\Projects",
        extension: "",
        parent: "D:\\",
        isDirectory: true,
      },
    ];
    const groups = buildGroups(
      apps,
      driveFiles,
      [],
      "",
      () => undefined,
      undefined,
      {
        text: "",
        appOnly: false,
        fileOnly: true,
        recentOnly: false,
        type: null,
        folder: null,
        pathRoot: "d:\\",
      },
    );

    expect(groups.map((group) => group.id)).toEqual(["file"]);
    expect(groups[0].items.map((item) => item.title)).toEqual([
      "Projects",
      "Project notes.md",
    ]);
    expect(groups[0].items[0].isDirectory).toBe(true);
  });

  it("does not truncate the first level of a drive scope", () => {
    const driveFiles = Array.from({ length: 9 }, (_, index) => ({
      id: `drive-folder-${index}`,
      name: `Folder ${index}`,
      path: `D:\\Folder ${index}`,
      extension: "",
      parent: "D:\\",
      isDirectory: true,
    }));
    const groups = buildGroups(
      apps,
      driveFiles,
      [],
      "",
      () => undefined,
      undefined,
      {
        text: "",
        appOnly: false,
        fileOnly: true,
        recentOnly: false,
        type: null,
        folder: null,
        pathRoot: "d:\\",
      },
    );

    expect(groups[0].items).toHaveLength(9);
  });
});
