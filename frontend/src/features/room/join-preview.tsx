import { useState } from "react";
import type { OpenRoomPreview } from "../../gen/OpenRoomPreview.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";

interface JoinPreviewProps {
  readonly preview: OpenRoomPreview;
  readonly joining: boolean;
  readonly joinError: string | null;
  readonly onJoin: () => void;
}

/** The classic join page: the channel's name, that you aren't a member, and Join. */
export function JoinPreview({ preview, joining, joinError, onJoin }: JoinPreviewProps) {
  const name = preview.name === "" ? "this channel" : `#${preview.name}`;

  return (
    <section className="room room-error enter-fade" aria-label={`Join ${name}`}>
      <h1 className="text-title">{name}</h1>
      <p className="text-muted">You're not a member of this channel.</p>
      {joinError === null ? null : <p className="text-muted">{joinError}</p>}
      <div className="room-error-actions">
        <Button variant="primary" loading={joining} onClick={onJoin}>
          Join channel
        </Button>
      </div>
    </section>
  );
}

/** The preview wired to `POST /rooms/:id/join`. Stays on screen until the room loads. */
export function JoinRoom({
  roomId,
  preview,
  focusMessageId,
}: {
  readonly roomId: number;
  readonly preview: OpenRoomPreview;
  readonly focusMessageId: number | null;
}) {
  const [joining, setJoining] = useState(false);
  const [joinError, setJoinError] = useState<string | null>(null);

  return (
    <JoinPreview
      preview={preview}
      joining={joining}
      joinError={joinError}
      onJoin={() => {
        setJoining(true);
        setJoinError(null);

        void actions.joinOpenRoom(roomId, focusMessageId).then(
          () => undefined,
          (failure: Error) => {
            setJoining(false);
            setJoinError(failure.message === "" ? "Couldn't join this channel." : failure.message);
          },
        );
      }}
    />
  );
}
