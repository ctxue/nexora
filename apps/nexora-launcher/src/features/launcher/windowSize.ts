export const LAUNCHER_WINDOW_HEIGHT = 620;
const LAUNCHER_WINDOW_HORIZONTAL_SPACE = 80;

export function launcherWindowWidth(searchWidth: number): number {
  return searchWidth + LAUNCHER_WINDOW_HORIZONTAL_SPACE;
}
