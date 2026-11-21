import { describe, expect, it } from "vitest";

import { settingsFromExport } from "./transfer";

describe("configuration transfer", () => {
  it("loads a settings-only configuration with a custom results height", () => {
    const loaded = settingsFromExport(
      JSON.stringify({ settings: { resultsMaxHeight: 480 } }),
    );

    expect(loaded).not.toBeNull();
    expect(loaded?.settings).toMatchObject({ resultsMaxHeight: 480 });
  });
});
