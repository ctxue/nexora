import {
  AppWindow,
  File,
  FileArchive,
  FileCode2,
  FileImage,
  FileSpreadsheet,
  FileText,
  Music,
  Presentation,
  Settings2,
  Video,
  type LucideIcon,
} from "lucide-react";
import type { CSSProperties } from "react";

interface FileVisual {
  icon: LucideIcon;
  color: string;
  background: string;
}

const FILE_VISUALS: Record<string, FileVisual> = {
  pdf: { icon: FileText, color: "#dc2626", background: "#fef2f2" },
  doc: { icon: FileText, color: "#2563eb", background: "#eff6ff" },
  docx: { icon: FileText, color: "#2563eb", background: "#eff6ff" },
  rtf: { icon: FileText, color: "#2563eb", background: "#eff6ff" },
  txt: { icon: FileText, color: "#475569", background: "#f8fafc" },
  md: { icon: FileText, color: "#475569", background: "#f8fafc" },
  xls: { icon: FileSpreadsheet, color: "#15803d", background: "#f0fdf4" },
  xlsx: { icon: FileSpreadsheet, color: "#15803d", background: "#f0fdf4" },
  csv: { icon: FileSpreadsheet, color: "#15803d", background: "#f0fdf4" },
  ppt: { icon: Presentation, color: "#ea580c", background: "#fff7ed" },
  pptx: { icon: Presentation, color: "#ea580c", background: "#fff7ed" },
  png: { icon: FileImage, color: "#7c3aed", background: "#f5f3ff" },
  jpg: { icon: FileImage, color: "#7c3aed", background: "#f5f3ff" },
  jpeg: { icon: FileImage, color: "#7c3aed", background: "#f5f3ff" },
  gif: { icon: FileImage, color: "#7c3aed", background: "#f5f3ff" },
  webp: { icon: FileImage, color: "#7c3aed", background: "#f5f3ff" },
  svg: { icon: FileImage, color: "#7c3aed", background: "#f5f3ff" },
  zip: { icon: FileArchive, color: "#b45309", background: "#fffbeb" },
  rar: { icon: FileArchive, color: "#b45309", background: "#fffbeb" },
  "7z": { icon: FileArchive, color: "#b45309", background: "#fffbeb" },
  gz: { icon: FileArchive, color: "#b45309", background: "#fffbeb" },
  ts: { icon: FileCode2, color: "#0891b2", background: "#ecfeff" },
  tsx: { icon: FileCode2, color: "#0891b2", background: "#ecfeff" },
  js: { icon: FileCode2, color: "#ca8a04", background: "#fefce8" },
  jsx: { icon: FileCode2, color: "#ca8a04", background: "#fefce8" },
  json: { icon: FileCode2, color: "#ca8a04", background: "#fefce8" },
  css: { icon: FileCode2, color: "#2563eb", background: "#eff6ff" },
  html: { icon: FileCode2, color: "#ea580c", background: "#fff7ed" },
  py: { icon: FileCode2, color: "#0891b2", background: "#ecfeff" },
  rs: { icon: FileCode2, color: "#b45309", background: "#fffbeb" },
  mp4: { icon: Video, color: "#db2777", background: "#fdf2f8" },
  mov: { icon: Video, color: "#db2777", background: "#fdf2f8" },
  mkv: { icon: Video, color: "#db2777", background: "#fdf2f8" },
  mp3: { icon: Music, color: "#9333ea", background: "#faf5ff" },
  wav: { icon: Music, color: "#9333ea", background: "#faf5ff" },
  flac: { icon: Music, color: "#9333ea", background: "#faf5ff" },
  exe: { icon: AppWindow, color: "#2563eb", background: "#eff6ff" },
  msi: { icon: AppWindow, color: "#2563eb", background: "#eff6ff" },
  ini: { icon: Settings2, color: "#64748b", background: "#f8fafc" },
  cfg: { icon: Settings2, color: "#64748b", background: "#f8fafc" },
  toml: { icon: Settings2, color: "#64748b", background: "#f8fafc" },
  yml: { icon: Settings2, color: "#64748b", background: "#f8fafc" },
  yaml: { icon: Settings2, color: "#64748b", background: "#f8fafc" },
};

const DEFAULT_VISUAL: FileVisual = {
  icon: File,
  color: "#64748b",
  background: "#f8fafc",
};

export function FileGlyph({ extension }: { extension: string }) {
  const visual = FILE_VISUALS[extension.toLowerCase()] ?? DEFAULT_VISUAL;
  const Icon = visual.icon;

  return (
    <span
      className="glyph file-glyph"
      style={
        {
          // Custom properties let the dark theme tint the same hue instead of
          // showing the light pastel chip.
          "--glyph-color": visual.color,
          "--glyph-bg": visual.background,
        } as CSSProperties
      }
    >
      <Icon size={13} strokeWidth={1.9} />
    </span>
  );
}
