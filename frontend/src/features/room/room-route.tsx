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
import { RightPane } from "../panes/right-pane.tsx";
import { usePhoneLayout, useRightPaneView, useRoomPaneLifecycle } from "../panes/use-right-pane.ts";
import { RoomSettingsHost } from "../rooms/room-settings-host.tsx";
import { prefetchThreadMemberships } from "../threads/prefetch.ts";
import { JoinRoom } from "./join-preview.tsx";
import { permalinkTarget, type RoomAccess } from "./message-destination.ts";
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
    let live = true;
    let opened = false;
    let stopWatching = () => {};

    const open = (focus: number | null) => {
      opened = true;
      void actions.openRoom(roomId, focus);
    };

    const dropAnchor = () => {
      void navigate({ to: "/r/$roomId", params: { roomId }, replace: true });
    };

    if (focusMessageId === null) {
      open(null);
    } else {
      const focus = focusMessageId;

      // After a preview joins, the message can be read. A reply leaves for its thread. A
      // still-missing message drops the anchor. A root message of this room is already the
      // visit's focus, so the join loads around it.
      const followJoinedPermalink = () => {
        const stop = store.subscribe(() => {
          if (!live || store.getState().rooms[roomId]?.detail == null) {
            return;
          }

          stop();
          void actions.messages.read(focus).then(
            ({ message }) => {
              if (!live) {
                return;
              }

              const target = permalinkTarget(roomId, focus, "member", {
                status: "found",
                message,
              });

              if (target.kind === "redirect") {
                router.history.replace(target.href);
              }
            },
            () => {
              if (live) {
                dropAnchor();
              }
            },
          );
        });

        stopWatching = stop;
      };

      const holdMissing = (access: RoomAccess) => {
        if (!live) {
          return;
        }

        const target = permalinkTarget(roomId, focus, access, { status: "missing" });

        if (target.kind === "focus" && target.messageId === null) {
          dropAnchor();

          return;
        }

        open(focus);
        followJoinedPermalink();
      };

      const settleMissing = () => {
        if (!live) {
          return;
        }

        const access = roomAccess(roomId);

        if (access !== null) {
          holdMissing(access);

          return;
        }

        let settled = false;

        const stop = store.subscribe(() => {
          const known = roomAccess(roomId);

          if (known === null || settled) {
            return;
          }

          settled = true;
          stop();
          holdMissing(known);
        });

        stopWatching = () => {
          settled = true;
          stop();
        };

        const known = roomAccess(roomId);

        if (known !== null) {
          settled = true;
          stop();
          holdMissing(known);
        }
      };

      // The timeline API only returns a message that is on this room. A message from another
      // room (or a reply) would 404 the page; open it where it actually is, or drop the anchor.
      // A not-yet-joined preview 404s the same read, so that failure keeps the permalink until
      // the join.
      void actions.messages.read(focus).then(
        ({ message }) => {
          if (!live) {
            return;
          }

          const target = permalinkTarget(roomId, focus, "member", { status: "found", message });

          if (target.kind === "redirect") {
            router.history.replace(target.href);

            return;
          }

          open(target.messageId);
        },
        () => {
          settleMissing();
        },
      );
    }

    return () => {
      live = false;
      stopWatching();

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
