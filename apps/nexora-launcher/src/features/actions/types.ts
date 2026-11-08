export type ResultActionId =
  | "copy-path"
  | "reveal-path"
  | "open-with"
  | "run-as-admin"
  | "preview-file"
  | "activate-window"
  | "minimize-window"
  | "close-window";

export interface ResultActionDescriptor {
  id: ResultActionId;
  label: string;
  shortcut?: string;
  danger?: boolean;
}
