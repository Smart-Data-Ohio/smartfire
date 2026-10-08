import { useEffect, useState } from "react";
import { useReducedMotion } from "../../motion/reduced-motion.ts";
import { Skeleton } from "../../ui/skeleton.tsx";
import { PaneListSkeleton } from "../panes/pane-states.tsx";
import "../destinations/destinations.css";
import "../panes/panes.css";

/**
 * How long a chunk may take before a placeholder is worth drawing. Fast loads then swap straight
 * to the screen. Long enough to cover a warm cache, short enough that a cold chunk doesn't look
 * stuck on a blank pane.
 */
export const ROUTE_PENDING_DELAY_MS = 150;

/**
 * Whether the placeholder may draw. Reduced motion shows it at once (the wait is a timed reveal,
 * and the bones already stay still). Otherwise it waits out {@link ROUTE_PENDING_DELAY_MS}.
 */
export function usePendingVisible(): boolean {
  const reduced = useReducedMotion();
  const [visible, setVisible] = useState(reduced);

  useEffect(() => {
    if (reduced) {
      setVisible(true);

      return;
    }

    const timeout = window.setTimeout(() => setVisible(true), ROUTE_PENDING_DELAY_MS);

    return () => window.clearTimeout(timeout);
  }, [reduced]);

  return visible;
}

/**
 * The main pane while a lazy route's chunk is still loading.
 *
 * TanStack wraps a match in Suspense when that route has a pending component, and this is the
 * fallback. `React.lazy` suspends during render, which never marks the match pending, so
 * `pendingMs` does not postpone it — the wait lives here. A chunk that arrives first replaces
 * the pane before anything is drawn. The shell renders this match inside its outlet, so the
 * rail, sidebar and the other panes stay mounted.
 */
export function RoutePending() {
  const visible = usePendingVisible();

  if (!visible) {
    return null;
  }

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
