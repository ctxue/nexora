import { describe, expect, it } from "vitest";

import { clipboardUiReducer, INITIAL_CLIPBOARD_UI } from "./ui";

describe("clipboard UI state", () => {
  it("keeps the clipboard open while clear confirmation is active", () => {
    const open = clipboardUiReducer(INITIAL_CLIPBOARD_UI, { type: "open" });
    const confirming = clipboardUiReducer(open, { type: "request-clear" });

    expect(confirming).toEqual({
      open: true,
      selectedIndex: 0,
      clearConfirmationOpen: true,
    });
  });

  it("keeps the clipboard open after history is cleared", () => {
    const confirming = clipboardUiReducer(
      { open: true, selectedIndex: 2, clearConfirmationOpen: true },
      { type: "confirm-clear" },
    );

    expect(confirming.open).toBe(true);
    expect(confirming.clearConfirmationOpen).toBe(false);
  });

  it("returns to launcher state only when the clipboard closes", () => {
    const closed = clipboardUiReducer(
      { open: true, selectedIndex: 2, clearConfirmationOpen: true },
      { type: "close" },
    );

    expect(closed).toEqual(INITIAL_CLIPBOARD_UI);
  });
});
