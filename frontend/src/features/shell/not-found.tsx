import { Link, useLocation } from "@tanstack/react-router";
import { useEffect } from "react";
import { unportedClassicPage } from "../../lib/screens.ts";

/**
 * Any path the router has no route for. A destination the SPA hasn't ported yet (the screen map
 * names it) opens on its classic page with a full page load; anything else is a 404.
 */
export function NotFound() {
  const { pathname, searchStr } = useLocation();
  const classic = unportedClassicPage(pathname, searchStr);

  // `replace`, so Back skips the forwarding.
  useEffect(() => {
    if (classic !== null) {
      window.location.replace(classic);
    }
  }, [classic]);

  if (classic !== null) {
    return null;
  }

  return (
    <section className="room room-error enter-fade" aria-label="Page not found">
      <p className="text-title">This page doesn't exist</p>
      <p className="text-muted">The link may be wrong, or the page may have moved.</p>
      <Link to="/">Go home</Link>
    </section>
  );
}
