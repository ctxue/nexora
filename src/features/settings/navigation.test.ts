import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  readRequestedSettingsSection,
  requestSettingsSection,
} from "./navigation";

describe("settings section navigation", () => {
  let values: Map<string, string>;

  beforeEach(() => {
    values = new Map();
    vi.stubGlobal("window", {
      localStorage: {
        getItem: (key: string) => values.get(key) ?? null,
        setItem: (key: string, value: string) => values.set(key, value),
        removeItem: (key: string) => values.delete(key),
      },
    });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("passes an explicit settings page to the next open", () => {
    requestSettingsSection("features");

    expect(readRequestedSettingsSection()).toBe("features");
    expect(readRequestedSettingsSection()).toBeNull();
  });

  it("clears a previous page request when settings opens normally", () => {
    requestSettingsSection("features");
    requestSettingsSection(null);

    expect(readRequestedSettingsSection()).toBeNull();
  });

  it("ignores unknown page identifiers", () => {
    requestSettingsSection("features");
    values.set("nexora.settings.open-section.v1", "missing");

    expect(readRequestedSettingsSection()).toBeNull();
  });
});
