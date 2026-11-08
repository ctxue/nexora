import type { LauncherItem } from "../../core/types";
import type { ResultActionDescriptor } from "./types";

const PATH_ACTIONS: ResultActionDescriptor[] = [
  { id: "copy-path", label: "Copy path" },
  { id: "reveal-path", label: "Show in folder" },
  { id: "open-with", label: "Open with..." },
  { id: "run-as-admin", label: "Run as administrator" },
];

const WINDOW_ACTIONS: ResultActionDescriptor[] = [
  { id: "activate-window", label: "Switch to window" },
  { id: "minimize-window", label: "Minimize window" },
  { id: "close-window", label: "Close window", danger: true },
];

export function buildResultActions(item: LauncherItem): ResultActionDescriptor[] {
  if (item.kind === "window" && item.windowId) {
    return WINDOW_ACTIONS;
  }

  if (!item.path) return [];

  if (item.kind === "file") {
    const canRunAsAdmin = ["exe", "com", "bat", "cmd", "msi"].includes(
      item.extension?.toLocaleLowerCase() ?? "",
    );
    return [
      ...PATH_ACTIONS.slice(0, 3),
      { id: "preview-file", label: "Quick look", shortcut: "Ctrl+Space" },
      ...(canRunAsAdmin ? [PATH_ACTIONS.at(-1)!] : []),
    ];
  }

  if (item.kind === "app") {
    return PATH_ACTIONS;
  }

  return [];
}
