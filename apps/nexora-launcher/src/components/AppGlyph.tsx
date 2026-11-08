export function AppGlyph({ name }: { name: string }) {
  const letter = name.trim().charAt(0).toUpperCase() || "?";

  return <span className="glyph app-glyph">{letter}</span>;
}
