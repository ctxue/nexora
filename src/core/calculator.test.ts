import { describe, expect, it } from "vitest";

import { evaluateSmartQuery } from "./calculator";

describe("evaluateSmartQuery", () => {
  it("evaluates arithmetic with normal operator precedence", () => {
    expect(evaluateSmartQuery("2 + 3 * 4")?.value).toBe("14");
  });

  it("formats fractional arithmetic without floating-point noise", () => {
    expect(evaluateSmartQuery("0.1 + 0.2")?.value).toBe("0.3");
  });

  it("rejects division by zero instead of returning infinity", () => {
    expect(evaluateSmartQuery("12 / 0")).toBeNull();
  });

  it("converts currencies with an explicit offline reference note", () => {
    const result = evaluateSmartQuery("100 usd cny");

    expect(result?.value).toBe("724 CNY");
    expect(result?.subtitle).toContain("offline reference rate");
  });

  it("converts physical units", () => {
    expect(evaluateSmartQuery("5 km m")?.value).toBe("5,000 m");
    expect(evaluateSmartQuery("32 f c")?.value).toBe("0 °C");
  });

  it("adds durations to a wall-clock time across midnight", () => {
    expect(evaluateSmartQuery("23:30 + 2h")?.value).toBe("01:30");
    expect(evaluateSmartQuery("16:30 + 2h")?.value).toBe("18:30");
  });

  it("ignores ordinary search text", () => {
    expect(evaluateSmartQuery("quarterly report")).toBeNull();
  });
});
