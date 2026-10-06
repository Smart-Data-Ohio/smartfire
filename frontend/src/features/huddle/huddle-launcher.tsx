/**
 * The room header's call button, with who's already in the call: "Join huddle" in channels and
 * direct messages ("In huddle" once joined, which opens the call), "Join voice" and "Join stage"
 * in voice and stage rooms, which toggle ("Leave voice", "Leave stage").
 */
import { useStore } from "../../store/store.ts";
import { Button } from "../../ui/button.tsx";
import { Tooltip } from "../../ui/tooltip.tsx";
import { UNKNOWN_NAME } from "../people/people.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import { joinHint } from "./alerts.ts";
import { callController } from "./call-controller.ts";
import { activePhase, useCall } from "./call-store.ts";
import { useCallParticipants, useHuddlesAvailable } from "./presence.ts";

const STACK = 3;

interface Labels {
  readonly join: string;
  readonly active: string;
  readonly toggle: boolean;
}

function labelsFor(kind: string): Labels {
  if (kind === "stage") {
    return { join: "Join stage", active: "Leave stage", toggle: true };
  }

  if (kind === "voice") {
    return { join: "Join voice", active: "Leave voice", toggle: true };
  }

  return { join: "Join huddle", active: "In huddle", toggle: false };
}

/** Who's in the call, as a small avatar stack with their names in the tooltip. */
function CallStack({ roomId }: { readonly roomId: number }) {
  const participants = useCallParticipants(roomId);

  const names = useStore((state) =>
    participants.map((entry) => state.users[entry.userId]?.name ?? UNKNOWN_NAME).join(", "),
  );

  if (participants.length === 0) {
    return null;
  }

  const extra = participants.length - STACK;

  return (
    <Tooltip content={`In the call: ${names}`} placement="bottom">
      <span className="huddle-stack" role="img" aria-label={`In the call: ${names}`}>
        {participants.slice(0, STACK).map((entry) => (
          <UserAvatar key={entry.userId} userId={entry.userId} size={20} decorative />
        ))}
        {extra > 0 ? <span className="huddle-stack-more">+{extra}</span> : null}
      </span>
    </Tooltip>
  );
}

export function HuddleLauncher({ roomId }: { readonly roomId: number }) {
  const available = useHuddlesAvailable();
  const row = useStore((state) => state.sidebar.rows[roomId] ?? null);
  const detailName = useStore((state) => state.rooms[roomId]?.detail?.displayName ?? null);

  const callRoomId = useCall((state) => state.roomId);
  const phase = useCall((state) => state.phase);

  if (!available || row === null || row.room.kind === "board") {
    return null;
  }

  const labels = labelsFor(row.room.kind);
  const here = callRoomId === roomId;
  const active = here && activePhase(phase);
  const joining = here && phase === "connecting";
  const label = joining ? "Joining…" : active ? labels.active : labels.join;
  const name = detailName ?? row.displayName;

  const onClick = () => {
    if (active && labels.toggle) {
      void callController.leave();

      return;
    }

    void callController.join(roomId, name, joinHint(roomId));
  };

  return (
    <span className="huddle-launch">
      <CallStack roomId={roomId} />
      <Button
        variant={active ? "secondary" : "ghost"}
        size="sm"
        icon={active && labels.toggle ? "phone-off" : "headphones"}
        className="huddle-launcher"
        aria-pressed={active}
        data-active={active || undefined}
        onClick={onClick}
      >
        <span className="huddle-launcher-label">{label}</span>
      </Button>
    </span>
  );
}
