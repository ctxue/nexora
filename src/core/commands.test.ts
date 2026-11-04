import { describe, expect, it } from "vitest";

import {
  acceptsArgument,
  buildArguments,
  buildTarget,
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
  it("enters command scope when any action target needs the query", () => {
    expect(
      acceptsArgument(
        command({
          action: {
            ...command().action,
            kind: "open-url",
            target: "https://example.com/search?q={query}",
          },
        }),
      ),
    ).toBe(true);
  });

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

  it("fills URL templates with every supported placeholder", () => {
    expect(
      buildUrl(
        "https://example.com?q={query}&clip={clipboard}&sel={selected}&on={date}",
        "rust & tauri",
        {
          clipboard: "copied / text",
          selected: "selected text",
          date: "2026-11-02",
        },
      ),
    ).toBe(
      "https://example.com?q=rust%20%26%20tauri&clip=copied%20%2F%20text&sel=selected%20text&on=2026-11-02",
    );
  });

  it("quotes clipboard and selection values in program arguments", () => {
    expect(
      buildArguments("--note {clipboard} --selected {selected}", "", {
        clipboard: "hello world",
        selected: "selected text",
        date: "2026-11-02",
      }),
    ).toBe('--note "hello world" --selected "selected text"');
  });

  it("replaces unresolved values with an empty string", () => {
    expect(
      buildArguments("--note {clipboard}", "", {
        clipboard: "",
        selected: "",
        date: "2026-11-02",
      }),
    ).toBe('--note ""');
  });

  it("fills path targets without adding shell quotes", () => {
    expect(
      buildTarget("C:\\Notes\\{date}\\{query}.txt", "weekly plan", {
        clipboard: "",
        selected: "",
        date: "2026-11-02",
      }),
    ).toBe("C:\\Notes\\2026-11-02\\weekly plan.txt");
  });
});
