import { useCallback, useEffect, useRef, useState } from "react";

/**
 * Transient status message shown as a toast (export finished, settings
 * imported, …). Auto-hides after a few seconds.
 */
export function useNotice() {
  const [notice, setNotice] = useState<string | null>(null);
  const timer = useRef<number | null>(null);

  const showNotice = useCallback((message: string) => {
    setNotice(message);
    if (timer.current !== null) {
      window.clearTimeout(timer.current);
    }
    timer.current = window.setTimeout(() => {
      setNotice(null);
      timer.current = null;
    }, 3600);
  }, []);

  useEffect(
    () => () => {
      if (timer.current !== null) {
        window.clearTimeout(timer.current);
      }
    },
    [],
  );

  return { notice, showNotice };
}
