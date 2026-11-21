import { describe, expect, it, vi } from "vitest";

import { searchBrowserGroups } from "./browserSearch";
import type { BrowserEntry } from "./types";

const entries: BrowserEntry[] = [
  {
    id: "bookmark-1",
    kind: "bookmark",
    title: "MDN Web Docs",
    url: "https://developer.mozilla.org/",
    browserName: "Edge",
    browserExe: "C:\\Edge\\msedge.exe",
  },
  {
    id: "history-1",
    kind: "history",
    title: "GitHub Pull Requests",
    url: "https://github.com/pulls",
    browserName: "Chrome",
    browserExe: "C:\\Chrome\\chrome.exe",
    lastVisited: 10,
  },
  {
    id: "tab-1",
    kind: "tab",
    title: "GitHub · NEXORA",
    url: "https://github.com/ctxue/nexora",
    browserName: "Edge",
    browserExe: "C:\\Edge\\msedge.exe",
    tabId: 42,
    windowId: 7,
  },
];

describe("searchBrowserGroups", () => {
  it("keeps tabs, bookmarks, and history in separate groups", () => {
    const groups = searchBrowserGroups(
      entries,
      "github",
      { tabs: "Tabs", bookmarks: "Bookmarks", history: "History" },
      () => undefined,
    );

    expect(groups.map((group) => group.id)).toEqual(["tab", "history"]);
    expect(groups[0].items[0]).toMatchObject({
      kind: "tab",
      url: "https://github.com/ctxue/nexora",
      browserTabId: 42,
    });
  });

  it("matches bookmark URLs as well as titles", () => {
    const groups = searchBrowserGroups(
      entries,
      "mozilla",
      { tabs: "Tabs", bookmarks: "Bookmarks", history: "History" },
      () => undefined,
    );

    expect(groups.map((group) => group.id)).toEqual(["bookmark"]);
    expect(groups[0].items[0].title).toBe("MDN Web Docs");
  });

  it("opens the exact browser entry represented by a result", async () => {
    const open = vi.fn();
    const groups = searchBrowserGroups(
      entries,
      "nexora",
      { tabs: "Tabs", bookmarks: "Bookmarks", history: "History" },
      open,
    );

    await groups[0].items[0].run();

    expect(open).toHaveBeenCalledWith(entries[2]);
  });
});
