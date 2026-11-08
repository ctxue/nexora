export interface IndexFolder {
  id: string;
  path: string;
}

export interface IndexState {
  folders: IndexFolder[];
  updatedAt: number;
}

export const INDEX_FOLDERS_KEY = "nexora.indexes.v1";

function normalizeFolderPath(path: string): string {
  return path.trim().replace(/[\\/]+$/, "");
}

function normalizeState(value: unknown): IndexState {
  if (!value || typeof value !== "object") {
    return { folders: [], updatedAt: 0 };
  }

  const record = value as Partial<IndexState>;
  const seen = new Set<string>();
  const folders: IndexFolder[] = [];

  if (Array.isArray(record.folders)) {
    for (const entry of record.folders) {
      if (!entry || typeof entry !== "object") continue;
      const path = normalizeFolderPath(
        typeof (entry as IndexFolder).path === "string"
          ? (entry as IndexFolder).path
          : "",
      );
      if (!path) continue;

      const key = path.toLowerCase();
      if (seen.has(key)) continue;
      seen.add(key);

      folders.push({ id: `index:${key}`, path });
      if (folders.length >= 20) break;
    }
  }

  return {
    folders,
    updatedAt:
      typeof record.updatedAt === "number" ? record.updatedAt : Date.now(),
  };
}

export function loadIndexState(): IndexState {
  try {
    const raw = window.localStorage.getItem(INDEX_FOLDERS_KEY);
    return raw ? normalizeState(JSON.parse(raw)) : { folders: [], updatedAt: 0 };
  } catch {
    return { folders: [], updatedAt: 0 };
  }
}

export function saveIndexState(state: IndexState) {
  try {
    window.localStorage.setItem(INDEX_FOLDERS_KEY, JSON.stringify(state));
  } catch {
  }
}
