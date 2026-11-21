import { describe, expect, it } from "vitest";

import type { LauncherItem } from "../../core/types";
import { buildResultActions } from "./registry";

function item(patch: Partial<LauncherItem>): LauncherItem {
  return {
    id: "result",
    kind: "file",
    title: "Result",
    subtitle: "Subtitle",
    run: () => undefined,
    ...patch,
  };
}

describe("buildResultActions", () => {
  it("offers path and launch actions for apps and files", () => {
    const actions = buildResultActions(
      item({ kind: "app", path: "C:\\Apps\\Code.exe" }),
    );

    expect(actions.map((action) => action.id)).toEqual([
      "copy-path",
      "reveal-path",
      "open-with",
      "run-as-admin",
    ]);
  });

  it("adds preview only to file results", () => {
    const fileActions = buildResultActions(
      item({ path: "C:\\Report.pdf", extension: "pdf" }),
    );
    const appActions = buildResultActions(
      item({ kind: "app", path: "C:\\Apps\\Code.exe" }),
    );

    expect(fileActions.some((action) => action.id === "preview-file")).toBe(
      true,
    );
    expect(appActions.some((action) => action.id === "preview-file")).toBe(
      false,
    );
  });

  it("only offers administrator launch for executable files", () => {
    const document = buildResultActions(
      item({ path: "C:\\Report.pdf", extension: "pdf" }),
    );
    const executable = buildResultActions(
      item({ path: "C:\\Tools\\setup.exe", extension: "exe" }),
    );

    expect(document.some((action) => action.id === "run-as-admin")).toBe(
      false,
    );
    expect(executable.some((action) => action.id === "run-as-admin")).toBe(
      true,
    );
  });

  it("offers window management actions for window results", () => {
    const actions = buildResultActions(
      item({ kind: "window", windowId: "1234" }),
    );

    expect(actions.map((action) => action.id)).toEqual([
      "activate-window",
      "minimize-window",
      "close-window",
    ]);
    expect(actions.at(-1)?.danger).toBe(true);
  });

  it("does not show a panel for results with no capabilities", () => {
    expect(buildResultActions(item({ kind: "command", id: "settings" }))).toEqual(
      [],
    );
  });
});
