import {
  Clipboard,
  Download,
  Folder,
  FolderOpen,
  Globe,
  HardDrive,
  Link2,
  Monitor,
  Power,
  RefreshCw,
  Search,
  Settings2,
  SquareTerminal,
  Trash2,
  type LucideIcon,
} from "lucide-react";

import type { CommandIcon } from "../core/types";

const COMMAND_ICONS: Record<CommandIcon, LucideIcon> = {
  downloads: Download,
  desktop: Monitor,
  documents: FolderOpen,
  computer: HardDrive,
  trash: Trash2,
  clipboard: Clipboard,
  settings: Settings2,
  refresh: RefreshCw,
  power: Power,
  globe: Globe,
  search: Search,
  link: Link2,
  folder: Folder,
  terminal: SquareTerminal,
};

export function CommandGlyphIcon({
  icon,
  size = 13,
  strokeWidth = 1.9,
}: {
  icon: CommandIcon;
  size?: number;
  strokeWidth?: number;
}) {
  const Icon = COMMAND_ICONS[icon];
  return <Icon size={size} strokeWidth={strokeWidth} />;
}

export function CommandGlyph({ icon }: { icon: CommandIcon }) {
  return (
    <span className="glyph command-glyph">
      <CommandGlyphIcon icon={icon} />
    </span>
  );
}
