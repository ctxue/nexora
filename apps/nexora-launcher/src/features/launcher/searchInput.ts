interface SearchInputSyncState {
  externalValue: string;
  currentValue: string;
  scopeChanged: boolean;
  inputFocused: boolean;
}

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
