import type { ButtonHTMLAttributes } from "react";

type BadgeVariant = "secondary" | "outline";

interface BadgeProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: BadgeVariant;
}

export function Badge({
  variant = "secondary",
  className = "",
  type = "button",
  ...props
}: BadgeProps) {
  return (
    <button
      data-slot="badge"
      data-variant={variant}
      type={type}
      className={`ui-badge${className ? ` ${className}` : ""}`}
      {...props}
    />
  );
}
