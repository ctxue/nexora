import type { HTMLAttributes } from "react";

type KbdProps = HTMLAttributes<HTMLElement>;

export function Kbd({ className = "", ...props }: KbdProps) {
  return (
    <kbd
      data-slot="kbd"
      className={`ui-kbd${className ? ` ${className}` : ""}`}
      {...props}
    />
  );
}
