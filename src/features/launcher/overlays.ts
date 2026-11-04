import type { LauncherItem } from "../../core/types";

export interface LauncherOverlayState {
  actionItem: LauncherItem | null;
  actionIndex: number;
  previewItem: LauncherItem | null;
}

export type LauncherOverlayAction =
  | { type: "open-actions"; item: LauncherItem }
  | { type: "move-actions"; delta: number; count: number }
  | { type: "close-actions" }
  | { type: "open-preview"; item: LauncherItem }
  | { type: "close-preview" }
  | { type: "close-all" };

export const INITIAL_OVERLAY_STATE: LauncherOverlayState = {
  actionItem: null,
  actionIndex: 0,
  previewItem: null,
};

export function overlayReducer(
  state: LauncherOverlayState,
  action: LauncherOverlayAction,
): LauncherOverlayState {
  switch (action.type) {
    case "open-actions":
      return {
        actionItem: action.item,
        actionIndex: 0,
        previewItem: null,
      };
    case "move-actions":
      if (action.count <= 0) return { ...state, actionIndex: 0 };
      return {
        ...state,
        actionIndex: Math.max(
          0,
          Math.min(action.count - 1, state.actionIndex + action.delta),
        ),
      };
    case "close-actions":
      return { ...state, actionItem: null, actionIndex: 0 };
    case "open-preview":
      return {
        actionItem: null,
        actionIndex: 0,
        previewItem: action.item,
      };
    case "close-preview":
      return { ...state, previewItem: null };
    case "close-all":
      return { ...INITIAL_OVERLAY_STATE };
  }
}
