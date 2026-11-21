export type LauncherVisibilityEvent = "hidden" | "shown";

const LAUNCHER_REST_WIDTH = 650;
const LAUNCHER_ENTRY_OVERSHOOT = 68;

export function launcherCardMotionFor(
  event: LauncherVisibilityEvent,
  animationEnabled = true,
  width = LAUNCHER_REST_WIDTH,
) {
  if (!animationEnabled) {
    return {
      width,
      opacity: 1,
      transition: {
        width: { duration: 0 },
        opacity: { duration: 0 },
      },
    };
  }

  if (event === "hidden") {
    return {
      width: width + LAUNCHER_ENTRY_OVERSHOOT,
      opacity: 0,
      transition: {
        width: { duration: 0 },
        opacity: { duration: 0 },
      },
    };
  }

  return {
    width,
    opacity: 1,
    transition: {
      width: {
        type: "spring" as const,
        visualDuration: 0.28,
        bounce: 0.12,
      },
      opacity: {
        duration: 0.1,
        ease: "easeOut" as const,
      },
    },
  };
}
