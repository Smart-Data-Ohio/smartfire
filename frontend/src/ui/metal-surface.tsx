import { isMetalFxSupported, MetalFx } from "metal-fx";
import type { ReactElement } from "react";

interface MetalSurfaceProps {
  readonly theme: "light" | "dark";
  readonly render: (metal: "static" | "live") => ReactElement;
}

/**
 * The lazily loaded half of the metal button: metal-fx's silver ring and glow around the host
 * button. If WebGL2 can't start after all, the button keeps its static brushed-silver styling.
 */
export default function MetalSurface({ theme, render }: MetalSurfaceProps) {
  if (!isMetalFxSupported()) {
    return render("static");
  }

  return (
    <MetalFx className="metal-host" variant="button" preset="silver" theme={theme} strength={0.9}>
      {render("live")}
    </MetalFx>
  );
}
