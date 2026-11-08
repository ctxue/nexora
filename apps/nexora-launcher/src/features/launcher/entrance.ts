export type LauncherVisibilityEvent = "hidden" | "shown";

const LAUNCHER_REST_WIDTH = 680;
const LAUNCHER_ENTRY_WIDTH = 748;

const HIDDEN_MOTION = {
  width: LAUNCHER_ENTRY_WIDTH,
  opacity: 0,
  transition: {
    width: { duration: 0 },
    opacity: { duration: 0 },
  },
} as const;

const SHOWN_MOTION = {
  width: LAUNCHER_REST_WIDTH,
  opacity: 1,
  transition: {
    width: {
      type: "spring",
      visualDuration: 0.28,
      bounce: 0.12,
    },
    opacity: {
      duration: 0.1,
      ease: "easeOut",
    },
  },
} as const;

const STATIC_MOTION = {
  width: LAUNCHER_REST_WIDTH,
  opacity: 1,
  transition: {
    width: { duration: 0 },
    opacity: { duration: 0 },
  },
} as const;

export function launcherCardMotionFor(
  event: "hidden",
  animationEnabled?: true,
): typeof HIDDEN_MOTION;
export function launcherCardMotionFor(
  event: "shown",
  animationEnabled?: true,
): typeof SHOWN_MOTION;
export function launcherCardMotionFor(
  event: LauncherVisibilityEvent,
  animationEnabled: false,
): typeof STATIC_MOTION;
export function launcherCardMotionFor(
  event: LauncherVisibilityEvent,
  animationEnabled?: boolean,
): typeof HIDDEN_MOTION | typeof SHOWN_MOTION | typeof STATIC_MOTION;
export function launcherCardMotionFor(
  event: LauncherVisibilityEvent,
  animationEnabled = true,
) {
  if (!animationEnabled) return STATIC_MOTION;
  return event === "hidden" ? HIDDEN_MOTION : SHOWN_MOTION;
}
