import { type ReactNode, Suspense, useEffect, useId, useSyncExternalStore } from "react";
import { useResolvedTheme } from "../lib/appearance.ts";
import { useReducedMotion } from "../motion/reduced-motion.ts";
import { lazyForUpdate as lazy } from "../service-worker/lazy.ts";
import { beamOwner, claimBeam, releaseBeam, subscribeBeam } from "./beam-store.ts";
import "./effects.css";

interface BeamProps {
  /** Ask for the beam. If several surfaces ask, the most recent one gets it. */
  readonly active: boolean;
  /** Corner radius of the wrapped surface, in px, so the beam follows its edge. */
  readonly radius?: number;
  readonly children: ReactNode;
}

const BeamOverlay = lazy(() => import("./beam-overlay.tsx"));

/**
 * A light that travels around a surface's border while an agent works on it (Jakub Antalik's
 * border-beam). The beam is an overlay beside the children, so they never remount when the
 * effect's chunk arrives; under reduced motion it is a still 1 px agent-violet edge.
 */
export function Beam({ active, radius = 8, children }: BeamProps) {
  const id = useId();
  const owner = useSyncExternalStore(subscribeBeam, beamOwner, () => null);
  const reduced = useReducedMotion();
  const theme = useResolvedTheme();

  useEffect(() => {
    if (!active) {
      return;
    }

    claimBeam(id);

    return () => releaseBeam(id);
  }, [active, id]);

  const on = active && owner === id;
  const mode = on ? (reduced ? "static" : "live") : undefined;

  return (
    <div className="beam-host" data-beam={mode} style={{ "--beam-radius": `${radius}px` }}>
      {children}
      {mode === "live" ? (
        <Suspense fallback={null}>
          <BeamOverlay radius={radius} theme={theme} />
        </Suspense>
      ) : null}
    </div>
  );
}
