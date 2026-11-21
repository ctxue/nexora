export interface ClipboardUiState {
  open: boolean;
  selectedIndex: number;
  clearConfirmationOpen: boolean;
}

export const INITIAL_CLIPBOARD_UI: ClipboardUiState = {
  open: false,
  selectedIndex: 0,
  clearConfirmationOpen: false,
};

export type ClipboardUiAction =
  | { type: "open" }
  | { type: "close" }
  | { type: "select"; index: number }
  | { type: "move"; delta: number; itemCount: number }
  | { type: "clamp"; itemCount: number }
  | { type: "request-clear" }
  | { type: "cancel-clear" }
  | { type: "confirm-clear" };

function clampIndex(index: number, itemCount: number): number {
  if (itemCount <= 0) return 0;
  return Math.max(0, Math.min(index, itemCount - 1));
}

export function clipboardUiReducer(
  state: ClipboardUiState,
  action: ClipboardUiAction,
): ClipboardUiState {
  switch (action.type) {
    case "open":
      return { ...INITIAL_CLIPBOARD_UI, open: true };
    case "close":
      return INITIAL_CLIPBOARD_UI;
    case "select":
      return { ...state, selectedIndex: Math.max(0, action.index) };
    case "move":
      return {
        ...state,
        selectedIndex: clampIndex(
          state.selectedIndex + action.delta,
          action.itemCount,
        ),
      };
    case "clamp":
      return {
        ...state,
        selectedIndex: clampIndex(state.selectedIndex, action.itemCount),
      };
    case "request-clear":
      return state.open
        ? { ...state, clearConfirmationOpen: true }
        : state;
    case "cancel-clear":
    case "confirm-clear":
      return { ...state, clearConfirmationOpen: false };
  }
}
