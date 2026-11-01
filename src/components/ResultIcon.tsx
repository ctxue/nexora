import type { LauncherItem } from "../core/types";
import { AppGlyph } from "./AppGlyph";
import { CommandGlyph } from "./CommandGlyph";
import { FileGlyph } from "./FileGlyph";

type IconItem = Pick<
  LauncherItem,
  "kind" | "title" | "extension" | "commandIcon"
>;

interface ResultIconProps {
  item: IconItem;
  icon?: string;
}

export function ResultIcon({ item, icon }: ResultIconProps) {
  if (icon) {
    return (
      <img
        className="real-icon"
        src={icon}
        alt=""
        draggable={false}
        loading="lazy"
      />
    );
  }

  if (item.kind === "app") {
    return <AppGlyph name={item.title} />;
  }

  if (item.kind === "file") {
    return <FileGlyph extension={item.extension ?? ""} />;
  }

  return item.commandIcon ? <CommandGlyph icon={item.commandIcon} /> : null;
}
