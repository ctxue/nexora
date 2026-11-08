import { describe, expect, it } from "vitest";

import { launcherCardMotionFor } from "./entrance";

describe("launcher entrance motion", () => {
  it("arms the hidden launcher at 110% width for the next reveal", () => {
    const motion = launcherCardMotionFor("hidden");

    expect(motion.width).toBe(748);
    expect(motion.opacity).toBe(0);
  });

  it("springs the width back without scaling the contents", () => {
    const motion = launcherCardMotionFor("shown");

    expect(motion.width).toBe(680);
    expect(motion.opacity).toBe(1);
    expect(motion.transition.width.type).toBe("spring");
    expect(motion.transition.width.visualDuration).toBeLessThanOrEqual(0.4);
    expect(motion.transition.width.bounce).toBeGreaterThan(0);
  });

  it("shows the hidden launcher at its resting size when animation is disabled", () => {
    const motion = launcherCardMotionFor("hidden", false);

    expect(motion.width).toBe(680);
    expect(motion.opacity).toBe(1);
    expect(motion.transition.width).toEqual({ duration: 0 });
  });
});
