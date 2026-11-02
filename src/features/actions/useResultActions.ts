import { useMemo } from "react";

import type { LauncherItem } from "../../core/types";
import { useI18n } from "../../i18n";
import { buildResultActions } from "./registry";
import type { ResultActionId } from "./types";

const ACTION_LABEL_KEYS: Record<ResultActionId, string> = {
  "copy-path": "actions.copyPath",
  "reveal-path": "actions.revealPath",
  "open-with": "actions.openWith",
  "run-as-admin": "actions.runAsAdmin",
  "preview-file": "actions.preview",
  "activate-window": "actions.activateWindow",
  "minimize-window": "actions.minimizeWindow",
  "close-window": "actions.closeWindow",
};

export function useResultActions(item: LauncherItem | null) {
  const { t } = useI18n();
  return useMemo(
    () =>
      item
        ? buildResultActions(item).map((action) => ({
            ...action,
            label: t(ACTION_LABEL_KEYS[action.id]),
          }))
        : [],
    [item, t],
  );
}
