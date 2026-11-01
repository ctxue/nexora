import type { CSSProperties } from "react";

const APP_PALETTES = [
  { background: "#eef2ff", color: "#4f46e5", border: "#e0e7ff" },
  { background: "#eff6ff", color: "#2563eb", border: "#dbeafe" },
  { background: "#f0fdf4", color: "#15803d", border: "#dcfce7" },
  { background: "#fff7ed", color: "#c2410c", border: "#ffedd5" },
  { background: "#fdf2f8", color: "#be185d", border: "#fce7f3" },
  { background: "#f5f3ff", color: "#6d28d9", border: "#ede9fe" },
  { background: "#ecfeff", color: "#0e7490", border: "#cffafe" },
];

function paletteFor(name: string) {
  let hash = 0;
  for (let index = 0; index < name.length; index += 1) {
    hash = (hash * 31 + name.charCodeAt(index)) >>> 0;
  }
  return APP_PALETTES[hash % APP_PALETTES.length];
}

export function AppGlyph({ name }: { name: string }) {
  const palette = paletteFor(name);
  const letter = name.trim().charAt(0).toUpperCase() || "?";

  return (
    <span
      className="glyph app-glyph"
      style={
        {
          // The palette is applied through custom properties so the dark theme
          // can re-tint the same hues instead of using fixed pastel tiles.
          "--glyph-color": palette.color,
          "--glyph-bg": palette.background,
          "--glyph-border": palette.border,
        } as CSSProperties
      }
    >
      {letter}
    </span>
  );
}
