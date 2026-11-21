import { useEffect, useState } from "react";

import { platform } from "../../platform/tauri";
import { acceleratorFromEvent } from "./accelerator";

export function useShortcutRecorder(
  onShortcutChange: (shortcut: string) => Promise<boolean>,
) {
  const [recording, setRecording] = useState(false);

  useEffect(() => {
    if (!recording) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();

      if (event.key === "Escape") {
        setRecording(false);
        return;
      }

      const accelerator = acceleratorFromEvent(event);
      if (!accelerator) return;

      void onShortcutChange(accelerator).then((accepted) => {
        if (accepted) setRecording(false);
      });
    };

    window.addEventListener("keydown", handleKeyDown, true);
    return () => window.removeEventListener("keydown", handleKeyDown, true);
  }, [onShortcutChange, recording]);

  useEffect(() => {
    if (!recording) {
      void platform.setShortcutCapture(false);
      return;
    }

    void platform.setShortcutCapture(true);
    let unlisten: (() => void) | undefined;

    void platform
      .onShortcutCaptured((shortcut) => {
        void onShortcutChange(shortcut).then((accepted) => {
          if (accepted) setRecording(false);
        });
      })
      .then((stopListening) => {
        unlisten = stopListening;
      });

    const stopOnBlur = () => setRecording(false);
    window.addEventListener("blur", stopOnBlur);

    return () => {
      void platform.setShortcutCapture(false);
      unlisten?.();
      window.removeEventListener("blur", stopOnBlur);
    };
  }, [onShortcutChange, recording]);

  return {
    recording,
    startRecording: () => setRecording(true),
    stopRecording: () => setRecording(false),
    toggleRecording: () => setRecording((value) => !value),
  };
}
