import { Link, useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { Button } from "../../ui/button.tsx";
import { joinSuggestedRoom } from "./welcome-actions.ts";

interface RoomActionProps {
  readonly roomId: number;
  readonly name: string;
  readonly joined: boolean;
}

/**
 * Open for a room the viewer is in, Join for an open one they aren't (then into it). A failed
 * join says why beside the button.
 */
export function RoomAction({ roomId, name, joined }: RoomActionProps) {
  const navigate = useNavigate();
  const [joining, setJoining] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (joined) {
    return (
      <Link
        to="/r/$roomId"
        params={{ roomId }}
        className="button welcome-room-action"
        data-variant="secondary"
        data-size="sm"
        aria-label={`Open #${name}`}
      >
        Open
      </Link>
    );
  }

  const join = () => {
    setJoining(true);
    setError(null);
    joinSuggestedRoom(roomId).then(
      () => navigate({ to: "/r/$roomId", params: { roomId } }),
      (failure: Error) => {
        setJoining(false);
        setError(failure.message === "" ? "Couldn't join this channel." : failure.message);
      },
    );
  };

  return (
    <span className="welcome-room-join">
      <Button
        variant="primary"
        size="sm"
        className="welcome-room-action"
        loading={joining}
        aria-label={`Join #${name}`}
        onClick={join}
      >
        Join
      </Button>
      {error === null ? null : (
        <span className="welcome-room-error text-meta" role="alert">
          {error}
        </span>
      )}
    </span>
  );
}
