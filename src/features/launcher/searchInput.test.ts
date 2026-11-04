import { describe, expect, it } from "vitest";

import {
  shouldSelectSearchInput,
  shouldSyncSearchInput,
} from "./searchInput";

describe("shouldSyncSearchInput", () => {
  it("does not overwrite what the user is actively typing", () => {
    expect(
      shouldSyncSearchInput({
        externalValue: "c",
        currentValue: "ca",
        scopeChanged: false,
        inputFocused: true,
      }),
    ).toBe(false);
  });

  it("applies a programmatic clear while the input is focused", () => {
    expect(
      shouldSyncSearchInput({
        externalValue: "",
        currentValue: "calculator",
        scopeChanged: false,
        inputFocused: true,
      }),
    ).toBe(true);
  });

  it("applies a command scope change immediately", () => {
    expect(
      shouldSyncSearchInput({
        externalValue: "",
        currentValue: "web",
        scopeChanged: true,
        inputFocused: true,
      }),
    ).toBe(true);
  });

  it("resynchronizes an unfocused input after an external change", () => {
    expect(
      shouldSyncSearchInput({
        externalValue: "settings",
        currentValue: "",
        scopeChanged: false,
        inputFocused: false,
      }),
    ).toBe(true);
  });
});

describe("shouldSelectSearchInput", () => {
  it("never reselects text after the user is already typing", () => {
    expect(shouldSelectSearchInput(true)).toBe(false);
  });

  it("selects the field when focus first moves into it", () => {
    expect(shouldSelectSearchInput(false)).toBe(true);
  });
});
