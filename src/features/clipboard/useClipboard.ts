import { useCallback, useEffect, useState } from "react";

import { platform, type ClipboardEntry } from "../../platform/tauri";

export function useClipboard() {
  const [entries, setEntries] = useState<ClipboardEntry[]>([]);

  const refresh = useCallback(async () => {
    try {
      setEntries(await platform.listClipboard());
    } catch {
      setEntries([]);
    }
  }, []);

  useEffect(() => {
    void refresh();
    let mounted = true;
    let unlisten: (() => void) | undefined;

    void platform.onClipboardUpdated(() => {
      if (mounted) void refresh();
    }).then((dispose) => {
      if (mounted) unlisten = dispose;
      else dispose();
    });

    return () => {
      mounted = false;
      unlisten?.();
    };
  }, [refresh]);

  const copy = useCallback((text: string) => platform.copyClipboard(text), []);

  const togglePin = useCallback(
    async (id: string) => {
      await platform.toggleClipboardPin(id);
      await refresh();
    },
    [refresh],
  );

  const remove = useCallback(
    async (id: string) => {
      await platform.deleteClipboardItem(id);
      await refresh();
    },
    [refresh],
  );

  const clear = useCallback(async () => {
    await platform.clearClipboard();
    setEntries([]);
  }, []);

  return {
    entries,
    copy,
    togglePin,
    remove,
    clear,
    refresh,
  };
}
