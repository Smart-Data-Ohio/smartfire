import "react";

declare module "react" {
  /** Lets `style` carry CSS custom properties (`style={{ "--avatar-size": "36px" }}`). */
  interface CSSProperties {
    [property: `--${string}`]: string | number | undefined;
  }
}
