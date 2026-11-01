import { useCallback, useEffect, useState } from "react";

import { platform } from "../../platform/tauri";
import { FONT_SIZE_BASE } from "./defaults";
import { applyInitialAppearance, useResolvedTheme } from "./appearance";
import {
  loadSettings,
  parseSettings,
  saveSettings,
  SETTINGS_KEY,
} from "./storage";
import type { Settings } from "./types";

export { applyInitialAppearance };

/** Reads the configuration, keeps the two windows in sync, and applies it. */
export function useSettings() {
  const [settings, setSettings] = useState<Settings>(loadSettings);
  const resolvedTheme = useResolvedTheme(settings.theme);

  const updateSettings = useCallback((patch: Partial<Settings>) => {
    setSettings((current) => {
      const next = { ...current, ...patch };
      saveSettings(next);
      return next;
    });
  }, []);

  // Keep the launcher and the separate settings window in sync.
  useEffect(() => {
    const onStorage = (event: StorageEvent) => {
      if (event.key === SETTINGS_KEY && event.newValue) {
        setSettings(parseSettings(event.newValue));
      }
    };
    window.addEventListener("storage", onStorage);
    return () => window.removeEventListener("storage", onStorage);
  }, []);

  useEffect(() => {
    document.documentElement.dataset.font = settings.fontFamily;
  }, [settings.fontFamily]);

  useEffect(() => {
    document.documentElement.dataset.theme = resolvedTheme;
    // The settings window keeps native decorations, so the native title bar has
    // to be recolored to match the page instead of the system default.
    void platform.setAppTheme(resolvedTheme).catch(() => {});
  }, [resolvedTheme]);

  useEffect(() => {
    document.documentElement.style.setProperty(
      "--launcher-radius",
      `${settings.radius}px`,
    );
  }, [settings.radius]);

  useEffect(() => {
    document.documentElement.style.setProperty(
      "--shadow-scale",
      String(settings.shadow / 100),
    );
  }, [settings.shadow]);

  useEffect(() => {
    document.documentElement.style.setProperty(
      "--font-scale",
      String(settings.fontSize / FONT_SIZE_BASE),
    );
  }, [settings.fontSize]);

  return { settings, updateSettings, resolvedTheme };
}
