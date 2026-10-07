/**
 * What call notices and rings look like: the incoming-call card (Join, Dismiss; "left the
 * huddle" once the caller hangs up), the join and leave toasts beside the dock, and the banner
 * and sidebar pill for a call going on elsewhere ("Maya is in your huddle", Join).
 */
import { useStore } from "zustand";
import { Button } from "../../ui/button.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { callNotices, goJoin, incomingCalls } from "./alerts.ts";
import { activePhase, useCall } from "./call-store.ts";
import { bannerText, noticeStore } from "./notices.ts";
import { ringStore } from "./ring.ts";

export function RingBanner() {
  const ring = useStore(ringStore, (state) => state.ring);
  const phase = useStore(ringStore, (state) => state.phase);

  if (ring === null) {
    return null;
  }

  const ended = phase === "ended";

  return (
    <section
      className="huddle-ring"
      data-ended={ended || undefined}
      role={ended ? "status" : "alertdialog"}
      aria-labelledby="huddle-ring-title"
      aria-describedby="huddle-ring-description"
    >
      <span className="huddle-ring-icon" aria-hidden="true">
        <Icon name={ended ? "phone-off" : "headphones"} size={18} />
      </span>
      <span className="huddle-ring-text">
        <strong id="huddle-ring-title" className="huddle-ring-title">
          {ended ? `${ring.callerName} left the huddle` : `${ring.callerName} started a huddle`}
        </strong>
        <span id="huddle-ring-description" className="huddle-ring-description">
          {ended ? `Missed call in ${ring.roomName}` : `Join the huddle in ${ring.roomName}`}
        </span>
      </span>
      {ended ? (
        <IconButton icon="x" label="Close" size="sm" onClick={() => incomingCalls.hide()} />
      ) : (
        <span className="huddle-ring-actions">
          <Button variant="ghost" size="sm" onClick={() => incomingCalls.dismiss()}>
            Dismiss
          </Button>
          <Button
            variant="primary"
            size="sm"
            icon="headphones"
            data-autofocus
            onClick={() => incomingCalls.join()}
          >
            Join
          </Button>
        </span>
      )}
    </section>
  );
}

/** Join and leave toasts; the container is the live region, as in the classic layout. */
export function CallToasts() {
  const toasts = useStore(noticeStore, (state) => state.toasts);

  return (
    <div className="huddle-toasts" aria-live="polite">
      {toasts.map((toast) => (
        <div key={toast.id} className="huddle-toast" data-kind={toast.kind}>
          <Icon name={toast.kind === "join" ? "headphones" : "phone-off"} size={14} />
          <span className="huddle-toast-text">{toast.text}</span>
          <IconButton
            icon="x"
            label="Dismiss"
            size="sm"
            onClick={() => callNotices.dismissToast(toast.id)}
          />
        </div>
      ))}
    </div>
  );
}

/** Hidden while the viewer is in that very call (joining answers it). */
function useNotice(roomId: number) {
  const banner = useStore(noticeStore, (state) => state.banners[roomId] ?? null);
  const here = useCall((state) => state.roomId === roomId && activePhase(state.phase));

  return here ? null : banner;
}

/** "Maya is in your huddle", in that room's conversation. */
export function JoinBanner({ roomId }: { readonly roomId: number }) {
  const banner = useNotice(roomId);

  if (banner === null) {
    return null;
  }

  return (
    <div className="huddle-join-banner" role="status">
      <Icon name="headphones" size={16} />
      <span className="huddle-join-banner-text">{bannerText(banner.joiners)}</span>
      <Button
        variant="primary"
        size="sm"
        onClick={() => {
          callNotices.dismiss(roomId);
          void goJoin(roomId, banner.roomName);
        }}
      >
        Join
      </Button>
      <IconButton icon="x" label="Dismiss" size="sm" onClick={() => callNotices.dismiss(roomId)} />
    </div>
  );
}

/** The same notice under the room's sidebar row. */
export function JoinPill({ roomId }: { readonly roomId: number }) {
  const banner = useNotice(roomId);

  if (banner === null) {
    return null;
  }

  return (
    <div className="huddle-join-pill">
      <span className="huddle-join-pill-text">{bannerText(banner.joiners)}</span>
      <Button
        variant="primary"
        size="sm"
        onClick={() => {
          callNotices.dismiss(roomId);
          void goJoin(roomId, banner.roomName);
        }}
      >
        Join
      </Button>
    </div>
  );
}
