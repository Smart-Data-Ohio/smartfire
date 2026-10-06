/**
 * Live call presence in the sidebar, as Discord nests it: under a voice or stage room, who's in
 * its call (speaking rings while the viewer is in that call too: a plain CSS ring, never
 * voice-glow here); beside any other room, a small "call going on" mark; a stage that's
 * streaming gets a live dot.
 */
import { useStore } from "../../store/store.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import { UNKNOWN_NAME } from "../people/people.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import { callController } from "./call-controller.ts";
import { useCall } from "./call-store.ts";
import { useCallLive, useCallParticipants, useHuddlesAvailable } from "./presence.ts";

/** The user ids speaking in the viewer's call, if it's this room's. */
function useSpeakingUsers(roomId: number): ReadonlySet<number> {
  const key = useCall((state) =>
    state.roomId !== roomId
      ? ""
      : state.snapshot.participants
          .filter((participant) => participant.speaking)
          .map((participant) =>
            participant.local
              ? "self"
              : String(callController.userIdFor(participant.identity) ?? ""),
          )
          .join(","),
  );

  const viewerId = useStore((state) => state.me?.user.id ?? null);

  return new Set(
    key
      .split(",")
      .filter((part) => part !== "")
      .map((part) => (part === "self" ? (viewerId ?? 0) : Number(part))),
  );
}

function Participant({
  userId,
  speaking,
  muted,
}: {
  readonly userId: number;
  readonly speaking: boolean;
  readonly muted: boolean;
}) {
  const name = useStore((state) => state.users[userId]?.name ?? UNKNOWN_NAME);

  return (
    <li className="voice-participant" data-speaking={speaking || undefined}>
      <span className="voice-participant-avatar">
        <UserAvatar userId={userId} size={20} decorative />
      </span>
      <span className="voice-participant-name">{name}</span>
      {muted ? (
        <span className="voice-participant-muted" title="Muted by a host">
          <Icon name="mic-off" size={12} />
        </span>
      ) : null}
    </li>
  );
}

/** The people in a voice or stage room's call, nested under its sidebar row. */
export function VoiceParticipants({ roomId }: { readonly roomId: number }) {
  const available = useHuddlesAvailable();
  const participants = useCallParticipants(roomId);
  const speaking = useSpeakingUsers(roomId);

  if (!available || participants.length === 0) {
    return null;
  }

  return (
    <ul className="voice-participants" aria-label="In the call">
      {participants.map((entry) => (
        <Participant
          key={entry.userId}
          userId={entry.userId}
          speaking={speaking.has(entry.userId)}
          muted={entry.serverMuted}
        />
      ))}
    </ul>
  );
}

/** A room row's call mark: a live dot for a streaming stage, else headphones while a call runs. */
export function CallMark({ roomId }: { readonly roomId: number }) {
  const available = useHuddlesAvailable();
  const count = useCallParticipants(roomId).length;
  const live = useCallLive(roomId);

  if (!available || (count === 0 && !live)) {
    return null;
  }

  if (live) {
    return (
      <span className="sidebar-call-mark" data-live title="Live now">
        <span className="sidebar-live-dot" />
      </span>
    );
  }

  return (
    <span
      className="sidebar-call-mark"
      title={`${count} in the call`}
      role="img"
      aria-label={`${count} in the call`}
    >
      <Icon name="headphones" size={12} />
    </span>
  );
}
