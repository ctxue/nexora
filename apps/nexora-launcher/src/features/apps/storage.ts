export interface ManualApp {
  id: string;
  name: string;
  path: string;
}

/** localStorage key holding applications added by hand (portable apps). */
export const MANUAL_APPS_KEY = "nexora.apps.v1";

function nameFromPath(path: string): string {
  const file = path.split(/[\\/]/).pop() ?? path;
  const name = file.replace(/\.exe$/i, "").trim();
  return name || file;
}

/** Exposed so the settings page can name a folder scan result. */
export function appNameFromPath(path: string): string {
  return nameFromPath(path);
}

function normalizeApps(value: unknown): ManualApp[] {
  if (!Array.isArray(value)) return [];

  const seen = new Set<string>();
  const apps: ManualApp[] = [];

  for (const entry of value) {
    if (!entry || typeof entry !== "object") continue;

    const record = entry as Partial<ManualApp>;
    const path = typeof record.path === "string" ? record.path.trim() : "";
    if (!path) continue;

    const key = path.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);

    const name =
      typeof record.name === "string" && record.name.trim()
        ? record.name.trim()
        : nameFromPath(path);

    apps.push({ id: `manual:${key}`, name, path });
    if (apps.length >= 200) break;
  }

  return apps;
}

export function loadManualApps(): ManualApp[] {
  try {
    const raw = window.localStorage.getItem(MANUAL_APPS_KEY);
    return raw ? normalizeApps(JSON.parse(raw)) : [];
  } catch {
    return [];
  }
}

export function saveManualApps(apps: ManualApp[]) {
  try {
    window.localStorage.setItem(MANUAL_APPS_KEY, JSON.stringify(apps));
  } catch {
    // A full quota must not break app search.
  }
}

/** Merges hand-added apps into the scanned list, skipping duplicates. */
export function mergeManualApps(
  scanned: Array<{ id: string; name: string; path: string; source: string }>,
  manual: ManualApp[],
) {
  if (manual.length === 0) return scanned;

  const paths = new Set(scanned.map((app) => app.path.toLowerCase()));
  const names = new Set(scanned.map((app) => app.name.toLowerCase()));

  const extra = manual
    .filter((app) => {
      const path = app.path.toLowerCase();
      const name = app.name.toLowerCase();
      return !paths.has(path) && !names.has(name);
    })
    .map((app) => ({
      id: `app:${app.path}`,
      name: app.name,
      path: app.path,
      source: "Portable",
    }));

  return extra.length === 0 ? scanned : [...scanned, ...extra];
}

/** Merges apps found in the configured index folders into the scanned list. */
export function mergeIndexedApps(
  scanned: Array<{ id: string; name: string; path: string; source: string }>,
  indexed: Array<{ id: string; name: string; path: string; source: string }>,
) {
  if (indexed.length === 0) return scanned;

  const paths = new Set(scanned.map((app) => app.path.toLowerCase()));
  const names = new Set(scanned.map((app) => app.name.toLowerCase()));

  const extra = indexed
    .filter((app) => {
      const path = app.path.toLowerCase();
      const name = app.name.toLowerCase();
      return !paths.has(path) && !names.has(name);
    })
    .map((app) => ({
      id: app.id,
      name: app.name,
      path: app.path,
      source: "Index",
    }));

  return extra.length === 0 ? scanned : [...scanned, ...extra];
}
