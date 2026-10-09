import { useLocation, useNavigate, useParams, useRouter } from "@tanstack/react-router";
import { useEffect } from "react";
import { classicPageFor } from "../../lib/screens.ts";
import { store, useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { BoardView } from "../boards/board-view.tsx";
import { Composer } from "../composer/composer.tsx";
import { FizzyCardOverlay } from "../fizzy/fizzy-card-overlay.tsx";
import { JoinBanner } from "../huddle/call-alerts.tsx";
import { CallView } from "../huddle/call-view.tsx";
import { useCallViewCovers } from "../huddle/call-view-cover.ts";
import { RightPane } from "../panes/right-pane.tsx";
import { usePhoneLayout, useRightPaneView, useRoomPaneLifecycle } from "../panes/use-right-pane.ts";
import { RoomSettingsHost } from "../rooms/room-settings-host.tsx";
import { prefetchThreadMemberships } from "../threads/prefetch.ts";
import { JoinRoom } from "./join-preview.tsx";
import type { RoomAccess } from "./message-destination.ts";
import { followPermalink } from "./permalink-follow.ts";
import { RoomHeader } from "./room-header.tsx";
import { Timeline } from "./timeline.tsx";
import "./room.css";

/**
 * Member when the sidebar or the room detail says so. A ready sidebar that omits the room, or a
 * join preview, means the viewer has not joined. `null` while that is still loading.
 */
function roomAccess(roomId: number): RoomAccess | null {
  const state = store.getState();
  const room = state.rooms[roomId];

  if (room?.detail != null || state.sidebar.rows[roomId] !== undefined) {
    return "member";
  }

  const sidebarSettled = state.sidebar.status === "ready" || state.sidebar.status === "error";

  if (room?.preview != null || sidebarSettled) {
    return "unjoined";
  }

  return null;
}

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
  const navigate = useNavigate();
  const router = useRouter();

  useRoomPaneLifecycle(roomId);

  useEffect(() => {
    let opened = false;

    const open = (focus: number | null) => {
      opened = true;
      void actions.openRoom(roomId, focus);
    };

    if (focusMessageId === null) {
      open(null);

      return () => {
        if (opened) {
          actions.closeRoom(roomId);
        }
      };
    }

    const messageId = focusMessageId;

    const cancel = followPermalink({
      roomId,
      messageId,
      read: (id) => actions.messages.read(id),
      access: () => roomAccess(roomId),
      joined: () => store.getState().rooms[roomId]?.detail != null,
      subscribe: (onChange) => store.subscribe(onChange),
      onFocus: (focus) => {
        open(focus);
      },
      onRedirect: (href) => {
        router.history.replace(href);
      },
      onDrop: () => {
        void navigate({ to: "/r/$roomId", params: { roomId }, replace: true });
      },
    });

    return () => {
      cancel();

      if (opened) {
        actions.closeRoom(roomId);
      }
    };
  }, [roomId, focusMessageId, navigate, router]);

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
  // On a phone the open call view covers the conversation: what's under it can't take focus.
  const callCovers = useCallViewCovers(roomId);
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
        <div className="room-part" inert={callCovers}>
          <RoomHeader roomId={roomId} />
        </div>
        <CallView roomId={roomId} />
        <div className="room-part" inert={callCovers}>
          <JoinBanner roomId={roomId} />
          {kind === "board" ? (
            <BoardView roomId={roomId} />
          ) : (
            <>
              <Timeline roomId={roomId} focusMessageId={focusMessageId} />
              <Composer roomId={roomId} />
            </>
          )}
        </div>
      </section>
      <RightPane roomId={roomId} />
      <RoomSettingsHost roomId={roomId} />
    </div>
  );
}
