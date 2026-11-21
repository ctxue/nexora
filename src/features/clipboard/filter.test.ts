import { describe, expect, it } from "vitest";

import type { ClipboardEntry } from "../../platform/tauri";
import { buildClipboardPreview, filterClipboardEntries } from "./filter";

const entries: ClipboardEntry[] = [
  {
    id: "old",
    text: "first copied line\nsecond line",
    createdAt: 10,
    pinned: false,
  },
  {
    id: "new",
    text: "NEXORA release notes",
    createdAt: 30,
    pinned: false,
  },
  {
    id: "pinned",
    text: "Keep this reference",
    createdAt: 1,
    pinned: true,
  },
];

describe("filterClipboardEntries", () => {
  it("keeps pinned items first and sorts the rest by newest first", () => {
    expect(
      filterClipboardEntries(entries, "").map((entry) => entry.id),
    ).toEqual(["pinned", "new", "old"]);
  });

  it("matches case-insensitive text and newlines", () => {
    expect(
      filterClipboardEntries(entries, "SECOND line").map(
        (entry) => entry.id,
      ),
    ).toEqual(["old"]);
  });
});

describe("buildClipboardPreview", () => {
  it("replaces line breaks with spaces and limits long previews", () => {
    expect(buildClipboardPreview("first\nsecond\tthird", 80)).toBe(
      "first second third",
    );
    expect(buildClipboardPreview("abcdef", 4)).toBe("abcd…");
  });
});
