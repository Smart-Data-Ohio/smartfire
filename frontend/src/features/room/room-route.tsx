import { useParams } from "@tanstack/react-router";
import { useEffect } from "react";
import { classicUrlFor, withClassicBypass } from "../../lib/screens.ts";
import { useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Composer } from "../composer/composer.tsx";
import { RightPane } from "../panes/right-pane.tsx";
import { usePhoneLayout, useRightPaneView } from "../panes/use-right-pane.ts";
import { prefetchThreadMemberships } from "../threads/prefetch.ts";
import { RoomHeader } from "./room-header.tsx";
import { Timeline } from "./timeline.tsx";
import "./room.css";

/**
 * `/app/r/$roomId` (and its permalink child): opens the room on the sync engine while it's on
 * screen, then lays out header, timeline and composer. Switching rooms keys the pane, so each
 * conversation starts fresh and the header cross-fades in.
 */
export function RoomRoute() {
  const params = useParams({ strict: false });
  const roomId = params.roomId ?? 0;
  const focusMessageId = params.messageId ?? null;

  useEffect(() => {
    void actions.openRoom(roomId, focusMessageId);
  }, [roomId, focusMessageId]);

  useEffect(() => () => actions.closeRoom(roomId), [roomId]);

  return <RoomPane key={roomId} roomId={roomId} focusMessageId={focusMessageId} />;
}

interface RoomPaneProps {
  readonly roomId: number;
  readonly focusMessageId: number | null;
}

/** The classic page for where the SPA is now (`?classic=1`, so it doesn't send them back here). */
function classicPage(): string {
  const { pathname, search } = window.location;

  return withClassicBypass(classicUrlFor(pathname, search) ?? "/");
}

function RoomPane({ roomId, focusMessageId }: RoomPaneProps) {
  const status = useStore((state) => state.rooms[roomId]?.status ?? "loading");
  const error = useStore((state) => state.rooms[roomId]?.error ?? null);
  const kind = useStore((state) => state.rooms[roomId]?.detail?.room.kind ?? null);
  const paneOpen = useRightPaneView() !== null;
  const phone = usePhoneLayout();
  const covered = phone && paneOpen;

  // Learn which of the room's threads the viewer follows, so reply indicators can show unread.
  useEffect(() => {
    if (kind !== null && kind !== "direct") {
      prefetchThreadMemberships(roomId);
    }
  }, [roomId, kind]);

  // Boards aren't ported yet (their own route comes later): the classic board opens instead.
  useEffect(() => {
    if (kind === "board") {
      window.location.replace(classicPage());
    }
  }, [kind]);

  if (kind === "board") {
    return null;
  }

  if (status === "error") {
    return (
      <section className="room room-error enter-fade" aria-label="Room unavailable">
        <p className="text-title">This room isn't available</p>
        <p className="text-muted">
          {error ?? "It may have been deleted, or you may have left it."}
        </p>
        <div className="room-error-actions">
          <Button
            variant="secondary"
            onClick={() => void actions.reloadRoom(roomId, focusMessageId)}
          >
            Try again
          </Button>
          <a className="text-muted" href={classicPage()}>
            Open in classic
          </a>
        </div>
      </section>
    );
  }

  return (
    <div
      className={`room-layout${phone ? " t-page-slide" : ""}`}
      data-page={phone ? (paneOpen ? "2" : "1") : undefined}
      data-pane-open={paneOpen || undefined}
    >
      <section
        className={`room${phone ? " t-page" : ""}`}
        data-page-id={phone ? "1" : undefined}
        aria-label="Conversation"
        inert={covered}
      >
        <RoomHeader roomId={roomId} />
        <Timeline roomId={roomId} focusMessageId={focusMessageId} />
        <Composer roomId={roomId} />
      </section>
      <RightPane roomId={roomId} />
    </div>
  );
}
