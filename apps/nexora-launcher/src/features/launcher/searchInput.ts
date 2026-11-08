interface SearchInputSyncState {
  externalValue: string;
  currentValue: string;
  scopeChanged: boolean;
  inputFocused: boolean;
}

/**
 * The input owns its text while focused. Parent search updates must never
 * rewrite the DOM value in the middle of typing.
 */
export function shouldSyncSearchInput({
  externalValue,
  currentValue,
  scopeChanged,
  inputFocused,
}: SearchInputSyncState): boolean {
  if (scopeChanged || externalValue === "") return true;
  return !inputFocused && externalValue !== currentValue;
}

export function shouldSelectSearchInput(inputFocused: boolean): boolean {
  return !inputFocused;
}
