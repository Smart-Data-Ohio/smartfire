import { Link } from "@tanstack/react-router";

/**
 * Any path the router has no route for: a 404. Every classic page the screen map names is ported,
 * and the server sends a signed-in person from a ported classic page to its SPA URL, so
 * forwarding there would only loop back here.
 */
export function NotFound() {
  return <PageNotFound />;
}

/** A reached route whose record doesn't exist or isn't accessible. */
export function PageNotFound() {
  return (
    <section className="room room-error enter-fade" aria-label="Page not found">
      <p className="text-title">This page doesn't exist</p>
      <p className="text-muted">The link may be wrong, or the page may have moved.</p>
      <Link to="/">Go home</Link>
    </section>
  );
}
