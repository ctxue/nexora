import {
  normalizeCommands,
  type CustomCommand,
} from "../../core/commands";

/** localStorage key holding the user's custom commands. */
export const COMMANDS_KEY = "nexora.commands.v1";

export function parseCommands(raw: string): CustomCommand[] {
  try {
    return normalizeCommands(JSON.parse(raw));
  } catch {
    return [];
  }
}

export function loadCommands(): CustomCommand[] {
  try {
    const raw = window.localStorage.getItem(COMMANDS_KEY);
    return raw ? parseCommands(raw) : [];
  } catch {
    return [];
  }
}

export function saveCommands(commands: CustomCommand[]) {
  try {
    window.localStorage.setItem(COMMANDS_KEY, JSON.stringify(commands));
  } catch {
    // Commands are a convenience; a full storage quota must not break search.
  }
}
