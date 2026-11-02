import { describe, expect, it } from "vitest";

import {
  buildArguments,
  buildUrl,
  createCommand,
  parseQuery,
  quoteWindowsArgument,
  type CustomCommand,
} from "./commands";

function command(patch: Partial<CustomCommand> = {}): CustomCommand {
  return {
    ...createCommand(),
    keyword: "gh",
    title: "GitHub",
    action: {
      kind: "web-search",
      template: "https://github.com/search?q={query}",
      target: "",
      args: "",
      browser: "default",
      newWindow: false,
    },
    ...patch,
  };
}

describe("parseQuery", () => {
  it("enters a command scope after Tab", () => {
    const github = command();

    expect(parseQuery("gh\trust tauri", [github])).toEqual({
      scope: github,
      argument: "rust tauri",
    });
  });

  it("keeps spaces as normal search text rather than activating a command", () => {
    expect(parseQuery("gh rust tauri", [command()])).toEqual({
      scope: null,
      argument: "gh rust tauri",
    });
  });

  it("keeps a keyword alone in normal result search", () => {
    expect(parseQuery("gh", [command()])).toEqual({
      scope: null,
      argument: "gh",
    });
  });

  it("does not scope through a disabled command", () => {
    expect(parseQuery("gh\trust", [command({ enabled: false })])).toEqual({
      scope: null,
      argument: "gh\trust",
    });
  });
});

describe("command arguments", () => {
  it("percent-encodes text substituted into a URL template", () => {
    expect(
      buildUrl("https://example.com?q={query}", "rust & tauri"),
    ).toBe("https://example.com?q=rust%20%26%20tauri");
  });

  it("quotes only arguments that need Windows command-line quoting", () => {
    expect(buildArguments("--search {query}", "hello")).toBe("--search hello");
    expect(buildArguments("--search {query}", "hello world")).toBe(
      '--search "hello world"',
    );
  });

  it("escapes quotes and trailing backslashes in Windows arguments", () => {
    expect(quoteWindowsArgument('a "quoted" path\\')).toBe(
      '"a \\"quoted\\" path\\\\"',
    );
  });
});
