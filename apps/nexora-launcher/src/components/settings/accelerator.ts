/** Keys that only act as modifiers and must not end a recording. */
const MODIFIER_KEYS = new Set(["Control", "Alt", "Shift", "Meta"]);

/**
 * Turns a keyboard event into a Tauri accelerator such as `Ctrl+Alt+Space`.
 * Returns `null` while only modifiers are held (nothing to record yet).
 */
export function acceleratorFromEvent(event: KeyboardEvent): string | null {
  if (MODIFIER_KEYS.has(event.key) || event.key === "Escape") return null;

  const modifiers: string[] = [];
  if (event.ctrlKey || event.metaKey) modifiers.push("Ctrl");
  if (event.altKey) modifiers.push("Alt");
  if (event.shiftKey) modifiers.push("Shift");

  if (modifiers.length === 0) return null;

  const code = event.code;
  let key: string;
  if (code === "Space" || event.key === " " || event.key === "Spacebar") {
    key = "Space";
  } else if (/^Key[A-Z]$/.test(code)) {
    key = code.slice(3);
  } else if (/^Digit[0-9]$/.test(code)) {
    key = code.slice(5);
  } else if (/^F\d{1,2}$/.test(code)) {
    key = code;
  } else if (code.startsWith("Arrow")) {
    key = code.slice(5);
  } else if (event.key.length === 1) {
    key = event.key.toUpperCase();
  } else {
    key = event.key;
  }

  return `${modifiers.join("+")}+${key}`;
}
