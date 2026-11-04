import { describe, expect, it } from "vitest";

import { matchesFileFilters, parseSearchQuery } from "./query";

describe("parseSearchQuery", () => {
  it("extracts type and text filters", () => {
    expect(parseSearchQuery("type:pdf quarterly report")).toEqual({
      text: "quarterly report",
      appOnly: false,
      fileOnly: true,
      recentOnly: false,
      type: "pdf",
      folder: null,
    });
  });

  it("extracts application-only and download-folder filters", () => {
    expect(parseSearchQuery("app:code file:folder:downloads")).toEqual({
      text: "code",
      appOnly: true,
      fileOnly: true,
      recentOnly: false,
      type: null,
      folder: "downloads",
    });
  });

  it("recognizes recent-only searches", () => {
    expect(parseSearchQuery("recent: browser")).toEqual({
      text: "browser",
      appOnly: false,
      fileOnly: false,
      recentOnly: true,
      type: null,
      folder: null,
    });
  });
});

describe("matchesFileFilters", () => {
  const file = {
    path: "C:\\Users\\Ada\\Downloads\\report.PDF",
    extension: "pdf",
    parent: "Downloads",
  };

  it("matches file type case-insensitively", () => {
    expect(matchesFileFilters(file, { type: "PDF", folder: null })).toBe(true);
    expect(matchesFileFilters(file, { type: "docx", folder: null })).toBe(false);
  });

  it("matches common folder aliases", () => {
    expect(
      matchesFileFilters(file, { type: null, folder: "downloads" }),
    ).toBe(true);
    expect(matchesFileFilters(file, { type: null, folder: "documents" })).toBe(
      false,
    );
  });
});
