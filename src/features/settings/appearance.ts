import { useEffect, useState } from "react";

import { FONT_SIZE_BASE } from "./defaults";
import { loadSettings } from "./storage";
import type { ResolvedTheme, ThemeMode } from "./types";

function prefersDark(): boolean {
  return (
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-color-scheme: dark)").matches
  );
}

/**
 * Resolves the configured theme, following the operating system when the user
 * picked "system".
 */
export function useResolvedTheme(mode: ThemeMode): ResolvedTheme {
  const [systemTheme, setSystemTheme] = useState<ResolvedTheme>(() =>
    prefersDark() ? "dark" : "light",
  );

  useEffect(() => {
    if (typeof window.matchMedia !== "function") return;
    const query = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => setSystemTheme(query.matches ? "dark" : "light");
    onChange();
    query.addEventListener("change", onChange);
    return () => query.removeEventListener("change", onChange);
  }, []);

  return mode === "system" ? systemTheme : mode;
}

/**
 * Applies the stored appearance before the first render so a dark-theme launch
 * never flashes the light palette.
 */
export function applyInitialAppearance() {
  const settings = loadSettings();
  const dark =
    settings.theme === "dark" ||
    (settings.theme === "system" && prefersDark());

  document.documentElement.dataset.theme = dark ? "dark" : "light";
  document.documentElement.dataset.font = settings.fontFamily;
  document.documentElement.style.setProperty(
    "--launcher-radius",
    `${settings.radius}px`,
  );
  document.documentElement.style.setProperty(
    "--shadow-scale",
    String(settings.shadow / 100),
  );
  document.documentElement.style.setProperty(
    "--font-scale",
    String(settings.fontSize / FONT_SIZE_BASE),
  );
}
