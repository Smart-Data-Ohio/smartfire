import { useEffect } from "react";
import { Button } from "../../ui/button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import "./offline.css";

/**
 * What a page that couldn't load shows while the device is offline: the service worker serves it
 * in place of the page, at the page's own URL, so reloading retries that page. It reloads by itself
 * once the browser says the connection is back.
 */
export function OfflineScreen() {
  useEffect(() => {
    const retry = () => window.location.reload();

    window.addEventListener("online", retry);

    return () => window.removeEventListener("online", retry);
  }, []);

  return (
    <main className="offline enter-fade" aria-labelledby="offline-title">
      <span className="offline-icon">
        <Icon name="wifi-off" size={24} />
      </span>
      <h1 id="offline-title" className="text-title">
        You're offline
      </h1>
      <p className="text-muted">
        Smartfire can't reach the server. This page reloads when you're back online.
      </p>
      <Button variant="secondary" icon="refresh-cw" onClick={() => window.location.reload()}>
        Try again
      </Button>
    </main>
  );
}
