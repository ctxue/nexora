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
      pathRoot: null,
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
      pathRoot: null,
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
      pathRoot: null,
    });
  });

  it("turns a drive path with slashes into a file-search scope", () => {
    expect(parseSearchQuery("d:// project")).toEqual({
      text: "project",
      appOnly: false,
      fileOnly: true,
      recentOnly: false,
      type: null,
      folder: null,
      pathRoot: "d:\\",
    });
    expect(parseSearchQuery("d://")).toMatchObject({
      text: "",
      fileOnly: true,
      pathRoot: "d:\\",
    });
  });

  it("treats the first path segment as a browsable folder", () => {
    expect(parseSearchQuery("D://font")).toMatchObject({
      text: "",
      fileOnly: true,
      pathRoot: "d:\\font",
    });
    expect(parseSearchQuery("D://font report")).toMatchObject({
      text: "report",
      fileOnly: true,
      pathRoot: "d:\\font",
    });
  });

  it("keeps a bare drive letter as a drive result query", () => {
    expect(parseSearchQuery("d:")).toMatchObject({
      text: "d:",
      fileOnly: false,
      pathRoot: null,
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
    expect(
      matchesFileFilters(file, {
        type: "PDF",
        folder: null,
        pathRoot: null,
      }),
    ).toBe(true);
    expect(
      matchesFileFilters(file, {
        type: "docx",
        folder: null,
        pathRoot: null,
      }),
    ).toBe(false);
  });

  it("matches common folder aliases", () => {
    expect(
      matchesFileFilters(file, {
        type: null,
        folder: "downloads",
        pathRoot: null,
      }),
    ).toBe(true);
    expect(
      matchesFileFilters(file, {
        type: null,
        folder: "documents",
        pathRoot: null,
      }),
    ).toBe(false);
  });

  it("limits files to the requested drive root", () => {
    const dFile = {
      path: "D:\\Projects\\report.PDF",
      extension: "pdf",
      parent: "D:\\Projects",
    };
    const similarlyNamedRoot = {
      path: "D:\\Reports\\report.PDF",
      extension: "pdf",
      parent: "D:\\Reports",
    };

    expect(
      matchesFileFilters(dFile, {
        type: null,
        folder: null,
        pathRoot: "d:\\projects",
      }),
    ).toBe(true);
    expect(
      matchesFileFilters(similarlyNamedRoot, {
        type: null,
        folder: null,
        pathRoot: "d:\\projects",
      }),
    ).toBe(false);
  });
});
