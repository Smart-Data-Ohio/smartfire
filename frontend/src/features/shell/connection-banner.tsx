import { type ReactNode, useEffect, useState } from "react";
import { useStore } from "../../store/store.ts";
import { Spinner } from "../../ui/button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";

/** A blip shorter than this never shows a banner. */
const GRACE_MS = 1500;

/** How long "Back online" stays after a reconnect. */
const RECOVERED_MS = 2000;

type Banner = "hidden" | "reconnecting" | "offline" | "recovered";

const BANNER_CONTENT = {
  hidden: null,
  reconnecting: (
    <>
      <Spinner />
      Reconnecting…
    </>
  ),
  offline: (
    <>
      <Icon name="alert" size={14} />
      You're offline. Messages you send will go out when you reconnect.
    </>
  ),
  recovered: (
    <>
      <Icon name="check" size={14} />
      Back online
    </>
  ),
} as const satisfies Record<Banner, ReactNode>;

/**
 * The connection strip over the panes: it slides open (height from 0fr, no measuring) only once
 * a drop has lasted the grace period, says "Back online" briefly when the socket returns, and
 * slides shut. A quick blip shows nothing.
 */
export function ConnectionBanner() {
  const connection = useStore((state) => state.connection);
  const [banner, setBanner] = useState<Banner>("hidden");
  const down = connection === "reconnecting" || connection === "offline";

  useEffect(() => {
    if (down) {
      const timer = window.setTimeout(() => setBanner(connection), GRACE_MS);

      return () => window.clearTimeout(timer);
    }

    if (connection !== "online") {
      return;
    }

    setBanner((current) => (current === "hidden" ? "hidden" : "recovered"));

    const timer = window.setTimeout(() => setBanner("hidden"), RECOVERED_MS);

    return () => window.clearTimeout(timer);
  }, [connection, down]);

  const open = banner !== "hidden";

  return (
    <div className="connection-banner" data-open={open} data-tone={banner} role="status">
      <div className="connection-banner-inner">{BANNER_CONTENT[banner]}</div>
    </div>
  );
}
