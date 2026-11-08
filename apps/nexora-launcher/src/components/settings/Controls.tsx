import type { CSSProperties, ReactNode } from "react";

export function Toggle({
  checked,
  onClick,
  disabled = false,
  label,
}: {
  checked: boolean;
  onClick: () => void;
  disabled?: boolean;
  label: string;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      className={`toggle${checked ? " is-on" : ""}`}
      onClick={onClick}
    >
      <span className="toggle-knob" />
    </button>
  );
}

export function ShortcutKeys({ shortcut }: { shortcut: string }) {
  return (
    <span className="settings-shortcut-keys">
      {shortcut.split("+").map((part, index) => (
        <kbd key={`${part}-${index}`}>{part}</kbd>
      ))}
    </span>
  );
}

export interface SegmentOption<T extends string> {
  value: T;
  label: string;
  icon?: ReactNode;
}

export function Segmented<T extends string>({
  options,
  value,
  onChange,
  label,
  withIcons = false,
}: {
  options: Array<SegmentOption<T>>;
  value: T;
  onChange: (value: T) => void;
  label: string;
  withIcons?: boolean;
}) {
  return (
    <div
      className={`segmented${withIcons ? " theme-segment" : ""}`}
      role="group"
      aria-label={label}
    >
      {options.map((option) => {
        const active = option.value === value;
        return (
          <button
            key={option.value}
            type="button"
            className={`segment${active ? " is-active" : ""}`}
            aria-pressed={active}
            onClick={() => onChange(option.value)}
          >
            {option.icon}
            {option.label}
          </button>
        );
      })}
    </div>
  );
}

export function SliderRow({
  value,
  min,
  max,
  valueLabel,
  onChange,
  label,
}: {
  value: number;
  min: number;
  max: number;
  valueLabel: string;
  onChange: (value: number) => void;
  label: string;
}) {
  const fill = `${((value - min) / (max - min)) * 100}%`;

  return (
    <div className="slider-control">
      <input
        type="range"
        className="settings-slider"
        min={min}
        max={max}
        step={1}
        value={value}
        aria-label={label}
        onChange={(event) => onChange(Number(event.target.value))}
        style={{ "--fill": fill } as CSSProperties}
      />
      <span className="slider-value">{valueLabel}</span>
    </div>
  );
}
