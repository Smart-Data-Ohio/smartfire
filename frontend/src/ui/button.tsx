import {
  type ComponentPropsWithRef,
  type MouseEvent,
  type ReactElement,
  Suspense,
  useState,
} from "react";
import { useResolvedTheme } from "../lib/appearance.ts";
import { useReducedMotion } from "../motion/reduced-motion.ts";
import { lazyForUpdate as lazy } from "../service-worker/lazy.ts";
import { Icon, type IconName } from "./icons/icon.tsx";
import "./button.css";

export type ButtonVariant =
  | "primary"
  | "secondary"
  | "ghost"
  | "danger"
  | "icon"
  | "pill"
  | "link"
  | "metal";

export type ButtonSize = "sm" | "md" | "lg";

export interface ButtonProps extends ComponentPropsWithRef<"button"> {
  readonly variant?: ButtonVariant;
  readonly size?: ButtonSize;
  /**
   * Busy: the label swaps to a spinner (and `loadingLabel`, if given) in place, without changing
   * the button's width. The button stays focusable but ignores clicks, and says it is busy.
   */
  readonly loading?: boolean;
  readonly loadingLabel?: string;
  readonly icon?: IconName;
  readonly trailingIcon?: IconName;
}

const ICON_SIZE = { sm: 14, md: 16, lg: 18 } as const;

export function Spinner({ label }: { readonly label?: string }) {
  return label === undefined ? (
    <span className="spinner" aria-hidden="true" />
  ) : (
    <span className="spinner" role="img" aria-label={label} />
  );
}

/** Loads metal-fx (WebGL) only when a metal button is actually on screen. */
const MetalSurface = lazy(() => import("./metal-surface.tsx"));

export function Button({
  variant = "secondary",
  size = "md",
  loading = false,
  loadingLabel,
  icon,
  trailingIcon,
  type = "button",
  className,
  children,
  onClick,
  disabled,
  ...rest
}: ButtonProps) {
  const iconSize = ICON_SIZE[size];

  const handleClick = (event: MouseEvent<HTMLButtonElement>) => {
    if (loading) {
      event.preventDefault();

      return;
    }

    onClick?.(event);
  };

  const content = (
    <span className="button-stack">
      <span className={`button-label t-text-swap${loading ? " is-exit" : ""}`}>
        {icon === undefined ? null : <Icon name={icon} size={iconSize} />}
        {children}
        {trailingIcon === undefined ? null : <Icon name={trailingIcon} size={iconSize} />}
      </span>
      <span className={`button-busy t-text-swap${loading ? "" : " is-exit"}`} aria-hidden="true">
        <Spinner />
        {loadingLabel}
      </span>
    </span>
  );

  const button = (metal: "static" | "live" | undefined) => (
    <button
      {...rest}
      type={type}
      className={className === undefined ? "button" : `button ${className}`}
      data-variant={variant}
      data-size={size}
      data-metal={metal}
      disabled={disabled}
      aria-busy={loading || undefined}
      aria-disabled={loading || undefined}
      onClick={handleClick}
    >
      {content}
    </button>
  );

  if (variant !== "metal") {
    return button(undefined);
  }

  return <MetalButton render={button} />;
}

/**
 * The metal variant: brushed static silver first (no WebGL, no extra bytes), upgraded to the live
 * metal-fx ring once its chunk has loaded, when WebGL2 exists and motion is allowed.
 */
function MetalButton({ render }: { readonly render: (metal: "static" | "live") => ReactElement }) {
  const reduced = useReducedMotion();
  const theme = useResolvedTheme();
  const [supported] = useState(() => "WebGL2RenderingContext" in window);

  if (reduced || !supported) {
    return render("static");
  }

  return (
    <Suspense fallback={render("static")}>
      <MetalSurface theme={theme} render={render} />
    </Suspense>
  );
}
