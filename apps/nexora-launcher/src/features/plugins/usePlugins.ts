import { useCallback, useEffect, useState } from "react";

import { platform, type PluginManifest } from "../../platform/tauri";

const REV_KEY = "nexora.plugins.rev";
const DISABLED_KEY = "nexora.plugins.disabled.v1";

function loadDisabled(): string[] {
  try {
    const raw = window.localStorage.getItem(DISABLED_KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed)
      ? parsed.filter((id): id is string => typeof id === "string")
      : [];
  } catch {
    return [];
  }
}

function saveDisabled(ids: string[]) {
  try {
    window.localStorage.setItem(DISABLED_KEY, JSON.stringify(ids));
  } catch {
    // Ignore quota errors: a disabled plugin list is not critical.
  }
}

function bumpRevision() {
  try {
    window.localStorage.setItem(REV_KEY, String(Date.now()));
  } catch {
    // Ignore; the other window will refresh next time it opens.
  }
}

/** Installed plugins and their enabled state, shared by both windows. */
export function usePlugins() {
  const [plugins, setPlugins] = useState<PluginManifest[]>([]);
  const [disabled, setDisabled] = useState<string[]>(loadDisabled);

  const refresh = useCallback(async () => {
    try {
      setPlugins(await platform.listPlugins());
    } catch {
      setPlugins([]);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    const onStorage = (event: StorageEvent) => {
      if (event.key === DISABLED_KEY) {
        setDisabled(loadDisabled());
      } else if (event.key === REV_KEY) {
        void refresh();
      }
    };
    window.addEventListener("storage", onStorage);
    return () => window.removeEventListener("storage", onStorage);
  }, [refresh]);

  const install = useCallback(
    async (source: string) => {
      const manifest = await platform.installPlugin(source);
      bumpRevision();
      await refresh();
      return manifest;
    },
    [refresh],
  );

  const uninstall = useCallback(
    async (id: string) => {
      await platform.uninstallPlugin(id);
      bumpRevision();
      await refresh();
    },
    [refresh],
  );

  const toggle = useCallback((id: string) => {
    setDisabled((current) => {
      const next = current.includes(id)
        ? current.filter((entry) => entry !== id)
        : [...current, id];
      saveDisabled(next);
      return next;
    });
    bumpRevision();
  }, []);

  return {
    plugins,
    disabled,
    refresh,
    install,
    uninstall,
    toggle,
    isEnabled: (id: string) => !disabled.includes(id),
  };
}
