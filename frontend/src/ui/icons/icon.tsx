import { createElement } from "react";
import { ICONS } from "./icon-data.ts";

export type IconName = keyof typeof ICONS;

interface IconProps {
  readonly name: IconName;
  /** Rendered size in px; icons sit on 16 (dense chrome), 18 (toolbars) or 20 (rail). */
  readonly size?: number;
  readonly className?: string;
}

/** A decorative Lucide icon. It is always aria-hidden: the control around it carries the name. */
export function Icon({ name, size = 16, className }: IconProps) {
  return (
    <svg
      className={className === undefined ? "icon" : `icon ${className}`}
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={size <= 16 ? 2 : 1.75}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      {ICONS[name].map(([tag, attrs]) =>
        createElement(tag, { key: `${tag}:${Object.values(attrs).join(" ")}`, ...attrs }),
      )}
    </svg>
  );
}
