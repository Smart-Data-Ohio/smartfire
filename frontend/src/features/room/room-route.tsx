import { useLocation, useParams } from "@tanstack/react-router";
import { useEffect } from "react";
import { classicPageFor } from "../../lib/screens.ts";
import { useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { BoardView } from "../boards/board-view.tsx";
import { Composer } from "../composer/composer.tsx";
import { FizzyCardOverlay } from "../fizzy/fizzy-card-overlay.tsx";
import { JoinBanner } from "../huddle/call-alerts.tsx";
import { CallView } from "../huddle/call-view.tsx";
import { RightPane } from "../panes/right-pane.tsx";
import { usePhoneLayout, useRightPaneView, useRoomPaneLifecycle } from "../panes/use-right-pane.ts";
import { RoomSettingsHost } from "../rooms/room-settings-host.tsx";
import { prefetchThreadMemberships } from "../threads/prefetch.ts";
import { JoinRoom } from "./join-preview.tsx";
import { RoomHeader } from "./room-header.tsx";
import { Timeline } from "./timeline.tsx";
import "./room.css";

/**
 * `/app/r/$roomId` (and its permalink, thread and "Create Fizzy card" children): opens the room on
 * the sync engine while it's on screen, then lays out header, timeline and composer (a board shows
 * its posts instead). Switching rooms keys the pane, so each conversation starts fresh and the
 * header cross-fades in.
 */
export function RoomRoute() {
  const params = useParams({ strict: false });
  const roomId = params.roomId ?? 0;
  const focusMessageId = params.messageId ?? null;

  useRoomPaneLifecycle(roomId);

  useEffect(() => {
    void actions.openRoom(roomId, focusMessageId);
  }, [roomId, focusMessageId]);

  useEffect(() => () => actions.closeRoom(roomId), [roomId]);

  return (
    <>
      {/* Siblings need their own keys: sharing one confuses React when the room changes. */}
      <RoomPane key={`pane-${roomId}`} roomId={roomId} focusMessageId={focusMessageId} />
      <FizzyCardOverlay key={`fizzy-${roomId}`} roomId={roomId} />
    </>
  );
}

interface RoomPaneProps {
  readonly roomId: number;
  readonly focusMessageId: number | null;
}

/** The classic page for where the SPA is now (`?classic=1`, so it doesn't send them back here). */
function useClassicPage(): string {
  const { pathname, searchStr } = useLocation();

  return classicPageFor(pathname, searchStr);
}

function RoomPane({ roomId, focusMessageId }: RoomPaneProps) {
  const status = useStore((state) => state.rooms[roomId]?.status ?? "loading");
  const error = useStore((state) => state.rooms[roomId]?.error ?? null);
  const detail = useStore((state) => state.rooms[roomId]?.detail ?? null);
  const preview = useStore((state) => state.rooms[roomId]?.preview ?? null);
  const kind = detail?.room.kind ?? null;
  const paneOpen = useRightPaneView() !== null;
  const phone = usePhoneLayout();
  const covered = phone && paneOpen;
  const classicPage = useClassicPage();

  // Learn which of the room's threads the viewer follows, so reply indicators can show unread.
  useEffect(() => {
    if (kind !== null && kind !== "direct") {
      prefetchThreadMemberships(roomId);
    }
  }, [roomId, kind]);

  if (preview !== null && detail === null) {
    return <JoinRoom roomId={roomId} preview={preview} />;
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
          <a className="text-muted" href={classicPage}>
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
        <CallView roomId={roomId} />
        <JoinBanner roomId={roomId} />
        {kind === "board" ? (
          <BoardView roomId={roomId} />
        ) : (
          <>
            <Timeline roomId={roomId} focusMessageId={focusMessageId} />
            <Composer roomId={roomId} />
          </>
        )}
      </section>
      <RightPane roomId={roomId} />
      <RoomSettingsHost roomId={roomId} />
    </div>
  );
}
