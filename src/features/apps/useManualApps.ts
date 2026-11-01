import { useCallback, useEffect, useState } from "react";

import {
  MANUAL_APPS_KEY,
  loadManualApps,
  saveManualApps,
  type ManualApp,
} from "./storage";

/** Applications the user added by hand (portable apps, folders of exes). */
export function useManualApps() {
  const [apps, setApps] = useState<ManualApp[]>(loadManualApps);

  useEffect(() => {
    const onStorage = (event: StorageEvent) => {
      if (event.key === MANUAL_APPS_KEY) {
        setApps(loadManualApps());
      }
    };
    window.addEventListener("storage", onStorage);
    return () => window.removeEventListener("storage", onStorage);
  }, []);

  const addApps = useCallback((entries: Array<{ path: string; name: string }>) => {
    setApps((current) => {
      const next = [...current];
      const paths = new Set(next.map((app) => app.path.toLowerCase()));

      for (const entry of entries) {
        const path = entry.path.trim();
        if (!path) continue;

        const key = path.toLowerCase();
        if (paths.has(key)) continue;
        paths.add(key);

        next.push({
          id: `manual:${key}`,
          name: entry.name.trim() || path.split(/[\\/]/).pop() || path,
          path,
        });
      }

      saveManualApps(next);
      return next;
    });
  }, []);

  const removeApp = useCallback((id: string) => {
    setApps((current) => {
      const next = current.filter((app) => app.id !== id);
      saveManualApps(next);
      return next;
    });
  }, []);

  return { apps, addApps, removeApp };
}
