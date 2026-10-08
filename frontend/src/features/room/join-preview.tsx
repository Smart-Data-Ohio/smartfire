import { useState } from "react";
import type { OpenRoomPreview } from "../../gen/OpenRoomPreview.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";

/** "3 members", or "1 member". */
export function membersLabel(count: number): string {
  return count === 1 ? "1 member" : `${count} members`;
}

interface JoinPreviewProps {
  readonly preview: OpenRoomPreview;
  readonly joining: boolean;
  readonly joinError: string | null;
  readonly onJoin: () => void;
}

/**
 * The classic join page: the channel's name, that you aren't a member, and Join. Rooms have no
 * topic, so the member count is the only extra fact.
 */
export function JoinPreview({ preview, joining, joinError, onJoin }: JoinPreviewProps) {
  const name = preview.name === "" ? "this channel" : `#${preview.name}`;

  return (
    <section className="room room-error enter-fade" aria-label={`Join ${name}`}>
      <h1 className="text-title">{name}</h1>
      <p className="text-muted">You're not a member of this channel.</p>
      <p className="text-muted">{membersLabel(preview.memberCount)}</p>
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
}: {
  readonly roomId: number;
  readonly preview: OpenRoomPreview;
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

        void actions.joinOpenRoom(roomId).then(
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
