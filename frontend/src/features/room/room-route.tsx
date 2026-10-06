import { useParams } from "@tanstack/react-router";
import { useEffect } from "react";
import { useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Composer } from "../composer/composer.tsx";
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

function RoomPane({ roomId, focusMessageId }: RoomPaneProps) {
  const status = useStore((state) => state.rooms[roomId]?.status ?? "loading");
  const error = useStore((state) => state.rooms[roomId]?.error ?? null);

  if (status === "error") {
    return (
      <section className="room room-error enter-fade" aria-label="Room unavailable">
        <p className="text-title">This room isn't available</p>
        <p className="text-muted">
          {error ?? "It may have been deleted, or you may have left it."}
        </p>
        <Button variant="secondary" onClick={() => void actions.openRoom(roomId, focusMessageId)}>
          Try again
        </Button>
      </section>
    );
  }

  return (
    <section className="room" aria-label="Conversation">
      <RoomHeader roomId={roomId} />
      <Timeline roomId={roomId} focusMessageId={focusMessageId} />
      <Composer roomId={roomId} />
    </section>
  );
}
