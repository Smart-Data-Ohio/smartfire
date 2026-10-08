import { useEffect, useState } from "react";
import { useReducedMotion } from "../../motion/reduced-motion.ts";
import { Skeleton } from "../../ui/skeleton.tsx";
import { PaneListSkeleton } from "../panes/pane-states.tsx";
import "../destinations/destinations.css";
import "../panes/panes.css";

/**
 * How long a navigation may take before the placeholder is worth showing. The route loader holds
 * the current screen for this long; a faster chunk never hides it.
 */
export const ROUTE_PENDING_DELAY_MS = 150;

/** Once a placeholder is showing, do not keep it up any longer. */
export const ROUTE_PENDING_MIN_MS = 0;

/**
 * React will not reveal a committed fallback for about 300ms. Pane bones wait longer than that
 * hold, so a fast chunk is shown before they paint.
 */
const FALLBACK_SKELETON_DELAY_MS = 400;

/** Bones after the fallback has committed. Reduced motion shows them at once. */
export function usePendingVisible(): boolean {
  const reduced = useReducedMotion();
  const [visible, setVisible] = useState(reduced);

  useEffect(() => {
    if (reduced) {
      setVisible(true);

      return;
    }

    const timeout = window.setTimeout(() => setVisible(true), FALLBACK_SKELETON_DELAY_MS);

    return () => window.clearTimeout(timeout);
  }, [reduced]);

  return visible;
}

/**
 * A pane whose chunk is still loading. Suspense hides that pane and shows this; the shell stays
 * mounted. Route loaders only mount it after {@link ROUTE_PENDING_DELAY_MS}.
 */
export function RoutePending() {
  return (
    <section className="page" role="status" aria-busy="true" aria-label="Loading page">
      <header className="page-header">
        <div className="page-heading">
          <Skeleton width={18} height={18} radius="md" />
          <Skeleton width={120} height={14} />
        </div>
      </header>
      <div className="page-body">
        <PaneListSkeleton rows={6} />
      </div>
    </section>
  );
}
