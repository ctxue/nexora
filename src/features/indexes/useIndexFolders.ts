import { useCallback, useEffect, useState } from "react";

import {
  INDEX_FOLDERS_KEY,
  loadIndexState,
  saveIndexState,
  type IndexState,
} from "./storage";

export function useIndexFolders() {
  const [state, setState] = useState<IndexState>(loadIndexState);

  useEffect(() => {
    const onStorage = (event: StorageEvent) => {
      if (event.key === INDEX_FOLDERS_KEY) {
        setState(loadIndexState());
      }
    };
    window.addEventListener("storage", onStorage);
    return () => window.removeEventListener("storage", onStorage);
  }, []);

  const addFolders = useCallback((paths: string[]) => {
    setState((current) => {
      const next = [...current.folders];
      const known = new Set(next.map((folder) => folder.path.toLowerCase()));

      for (const raw of paths) {
        const path = raw.trim().replace(/[\\/]+$/, "");
        if (!path) continue;
        const key = path.toLowerCase();
        if (known.has(key)) continue;
        known.add(key);
        next.push({ id: `index:${key}`, path });
      }

      const state = { folders: next, updatedAt: Date.now() };
      saveIndexState(state);
      return state;
    });
  }, []);

  const removeFolder = useCallback((id: string) => {
    setState((current) => {
      const state = {
        folders: current.folders.filter((folder) => folder.id !== id),
        updatedAt: Date.now(),
      };
      saveIndexState(state);
      return state;
    });
  }, []);

  const refresh = useCallback(() => {
    setState((current) => {
      const state = { folders: current.folders, updatedAt: Date.now() };
      saveIndexState(state);
      return state;
    });
  }, []);

  return {
    folders: state.folders,
    updatedAt: state.updatedAt,
    addFolders,
    removeFolder,
    refresh,
  };
}
