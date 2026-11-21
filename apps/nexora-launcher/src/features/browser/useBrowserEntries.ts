import { useCallback, useEffect, useState } from "react";

import type { BrowserEntry } from "../../core/types";
import { platform } from "../../platform/tauri";
import type { Settings } from "../settings/types";

export function useBrowserEntries(settings: Settings) {
  const [entries, setEntries] = useState<BrowserEntry[]>([]);
  const enabled =
    settings.browserBookmarks ||
    settings.browserHistory ||
    settings.browserTabs;

  const refresh = useCallback(() => {
    if (!enabled) {
      setEntries([]);
      return;
    }
    void platform
      .listBrowserEntries(
        settings.browserBookmarks,
        settings.browserHistory,
        settings.browserTabs,
      )
      .then(setEntries)
      .catch(() => setEntries([]));
  }, [
    enabled,
    settings.browserBookmarks,
    settings.browserHistory,
    settings.browserTabs,
  ]);

  useEffect(() => {
    refresh();
    if (!enabled) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void platform.onLauncherShown(() => {
      if (!disposed) refresh();
    }).then((dispose) => {
      if (disposed) dispose();
      else unlisten = dispose;
    });
    const interval = window.setInterval(refresh, 30_000);
    return () => {
      disposed = true;
      unlisten?.();
      window.clearInterval(interval);
    };
  }, [enabled, refresh]);

  return { entries, refresh };
}
