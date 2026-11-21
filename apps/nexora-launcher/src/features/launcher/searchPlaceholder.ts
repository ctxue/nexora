interface SearchPlaceholderInput {
  enabled: boolean;
  custom: string;
  fallback: string;
  scoped: string | null;
}

export function searchPlaceholderFor({
  enabled,
  custom,
  fallback,
  scoped,
}: SearchPlaceholderInput): {
  placeholder: string | undefined;
  label: string;
} {
  const text = scoped ?? (custom.trim() || fallback);
  return {
    placeholder: enabled ? text : undefined,
    label: text,
  };
}
