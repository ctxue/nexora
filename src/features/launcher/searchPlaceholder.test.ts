import { describe, expect, it } from "vitest";

import { searchPlaceholderFor } from "./searchPlaceholder";

describe("search placeholder", () => {
  it("hides the placeholder by default without losing the accessible label", () => {
    expect(
      searchPlaceholderFor({
        enabled: false,
        custom: "",
        fallback: "Search apps",
        scoped: null,
      }),
    ).toEqual({ placeholder: undefined, label: "Search apps" });
  });

  it("uses custom text when enabled and trims surrounding whitespace", () => {
    expect(
      searchPlaceholderFor({
        enabled: true,
        custom: "  Type a command  ",
        fallback: "Search apps",
        scoped: null,
      }),
    ).toEqual({ placeholder: "Type a command", label: "Type a command" });
  });

  it("falls back to translated default text when custom text is blank", () => {
    expect(
      searchPlaceholderFor({
        enabled: true,
        custom: "   ",
        fallback: "Search apps",
        scoped: null,
      }).placeholder,
    ).toBe("Search apps");
  });

  it("keeps scope-specific hints when hints are enabled", () => {
    expect(
      searchPlaceholderFor({
        enabled: true,
        custom: "Type what you need",
        fallback: "Search apps",
        scoped: "Search with Edge",
      }),
    ).toEqual({
      placeholder: "Search with Edge",
      label: "Search with Edge",
    });
  });
});
