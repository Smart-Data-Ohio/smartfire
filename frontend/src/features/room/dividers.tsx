import { useStore } from "../../store/store.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import { UserAvatar } from "../people/user-avatar.tsx";
import { ROOM_KIND_ICON } from "./room-icon.ts";

/** A day boundary: a hairline with the day in a pill (Slack's). */
export function DayDivider({ label }: { readonly label: string }) {
  return (
    <div className="day-divider">
      <span className="day-divider-label">{label}</span>
    </div>
  );
}

/** Where this visit's unread messages start: Discord's red rule with a "New" tag. */
export function UnreadDivider({ count }: { readonly count: number }) {
  const label = count > 0 ? `${count} new ${count === 1 ? "message" : "messages"}` : "New";

  return (
    <div className="unread-divider">
      <span className="visually-hidden">{label}</span>
      <span className="unread-divider-label" aria-hidden="true">
        New
      </span>
    </div>
  );
}

/**
 * The start of a room's history (and, for a room with no messages, its whole empty state): a big
 * glyph or the other person's avatar and a line that says where you are.
 */
export function RoomIntro({ roomId }: { readonly roomId: number }) {
  const detail = useStore((state) => state.rooms[roomId]?.detail ?? null);
  const empty = useStore((state) => (state.timelines[roomId]?.ids.length ?? 0) === 0);

  if (detail === null) {
    return <div className="room-intro" />;
  }

  const { room, displayName, directMemberIds } = detail;
  const direct = room.kind === "direct";
  const otherId = direct ? directMemberIds[0] : undefined;

  return (
    <div className="room-intro enter-fade" data-empty={empty || undefined}>
      {otherId === undefined ? (
        <span className="room-intro-glyph" aria-hidden="true">
          <Icon name={ROOM_KIND_ICON[room.kind]} size={28} />
        </span>
      ) : (
        <UserAvatar userId={otherId} size={64} presence decorative />
      )}
      <h2 className="room-intro-title">{direct ? displayName : `Welcome to #${displayName}`}</h2>
      <p className="room-intro-text">
        {direct
          ? `This is the very beginning of your conversation with ${displayName}.`
          : `This is the very beginning of #${displayName}.`}
        {empty ? " No one has posted yet." : ""}
      </p>
    </div>
  );
}
