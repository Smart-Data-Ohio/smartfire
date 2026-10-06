import type { Placement } from "../lib/anchor.ts";
import { Button, type ButtonProps } from "./button.tsx";
import type { IconName } from "./icons/icon.tsx";
import { ariaKeyShortcuts } from "./kbd.tsx";
import { Tooltip } from "./tooltip.tsx";

export interface IconButtonProps
  extends Omit<ButtonProps, "variant" | "children" | "icon" | "trailingIcon" | "aria-label"> {
  readonly icon: IconName;
  /** Required: it is the button's accessible name and its tooltip. */
  readonly label: string;
  readonly shortcut?: readonly string[] | undefined;
  readonly tooltipPlacement?: Placement;
}

/**
 * An icon-only button. The label is both its accessible name and its tooltip, so an icon is
 * never left unexplained; a shortcut shows in the tooltip and is announced via aria-keyshortcuts.
 */
export function IconButton({
  icon,
  label,
  shortcut,
  tooltipPlacement = "top",
  ...rest
}: IconButtonProps) {
  return (
    <Tooltip content={label} shortcut={shortcut} placement={tooltipPlacement} describe={false}>
      <Button
        {...rest}
        variant="icon"
        icon={icon}
        aria-label={label}
        aria-keyshortcuts={shortcut === undefined ? undefined : ariaKeyShortcuts(shortcut)}
      />
    </Tooltip>
  );
}
