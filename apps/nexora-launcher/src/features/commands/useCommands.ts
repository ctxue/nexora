import { useCallback, useEffect, useState } from "react";

import type { CustomCommand } from "../../core/commands";
import {
  COMMANDS_KEY,
  loadCommands,
  parseCommands,
  saveCommands,
} from "./storage";

/** Custom commands shared by the launcher and the settings window. */
export function useCommands() {
  const [commands, setCommandsState] = useState<CustomCommand[]>(loadCommands);

  useEffect(() => {
    const onStorage = (event: StorageEvent) => {
      if (event.key !== COMMANDS_KEY) return;
      setCommandsState(event.newValue ? parseCommands(event.newValue) : []);
    };
    window.addEventListener("storage", onStorage);
    return () => window.removeEventListener("storage", onStorage);
  }, []);

  const setCommands = useCallback((next: CustomCommand[]) => {
    setCommandsState(next);
    saveCommands(next);
  }, []);

  return { commands, setCommands };
}
