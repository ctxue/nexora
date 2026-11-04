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

const FILE_ICONS: Record<string, LucideIcon> = {
  pdf: FileText,
  doc: FileText,
  docx: FileText,
  rtf: FileText,
  txt: FileText,
  md: FileText,
  xls: FileSpreadsheet,
  xlsx: FileSpreadsheet,
  csv: FileSpreadsheet,
  ppt: Presentation,
  pptx: Presentation,
  png: FileImage,
  jpg: FileImage,
  jpeg: FileImage,
  gif: FileImage,
  webp: FileImage,
  svg: FileImage,
  zip: FileArchive,
  rar: FileArchive,
  "7z": FileArchive,
  gz: FileArchive,
  ts: FileCode2,
  tsx: FileCode2,
  js: FileCode2,
  jsx: FileCode2,
  json: FileCode2,
  css: FileCode2,
  html: FileCode2,
  py: FileCode2,
  rs: FileCode2,
  mp4: Video,
  mov: Video,
  mkv: Video,
  mp3: Music,
  wav: Music,
  flac: Music,
  exe: AppWindow,
  msi: AppWindow,
  ini: Settings2,
  cfg: Settings2,
  toml: Settings2,
  yml: Settings2,
  yaml: Settings2,
};

export function FileGlyph({ extension }: { extension: string }) {
  const Icon = FILE_ICONS[extension.toLowerCase()] ?? File;

  return (
    <span className="glyph file-glyph">
      <Icon size={13} strokeWidth={1.9} />
    </span>
  );
}
