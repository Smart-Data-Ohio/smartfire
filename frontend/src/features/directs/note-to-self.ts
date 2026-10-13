/**
 * The note-to-self (Slack's "message yourself"): the direct room whose only member is you, for
 * notes, drafts and links. The server names it after you and offers it first among the sidebar's
 * placeholders until it exists; `POST /directs` with nobody else opens it, creating it on first
 * use. Everywhere it shows, it reads as "Your Name (you)".
 */
import { isNoteToSelf } from "../../store/note-to-self.ts";
import type { State } from "../../store/state.ts";
import { useStore } from "../../store/store.ts";

export { isNoteToSelf };

/** The muted tag after your name. */
export const YOU_TAG = "(you)";

/** The header's line and the empty room's intro. */
export const NOTE_TO_SELF_LINE =
  "This is your space. Draft messages, keep links, or jot notes for later.";

/** The composer's placeholder in it. */
export const NOTE_TO_SELF_PLACEHOLDER = "Jot something down";

/** "Your Name (you)", as plain text. */
export function withYou(name: string): string {
  return `${name} ${YOU_TAG}`;
}

/** Names people by their profiles in `users`, yours with "(you)" after it (switcher, search). */
export function namesWithYou(
  users: Readonly<Record<number, { readonly name: string }>>,
  viewerId: number | null,
): (userId: number) => string | undefined {
  return (userId) => {
    const name = users[userId]?.name;

    return userId === viewerId && name !== undefined ? withYou(name) : name;
  };
}

export function viewerIdOf(state: State): number | null {
  return state.me?.user.id ?? state.boot?.user.id ?? null;
}

/** Your own name, as the sidebar and headers show it. */
export function viewerNameOf(state: State): string | null {
  const id = viewerIdOf(state);

  return (id === null ? undefined : state.users[id]?.name) ?? state.me?.user.name ?? null;
}

const roomIds = new WeakMap<object, number | null>();

/** Your note-to-self's room id from the sidebar rows (hidden or not), or `null` before it exists. */
export function noteToSelfRoomId(state: State): number | null {
  const rows = state.sidebar.rows;
  const viewerId = viewerIdOf(state);

  if (viewerId === null) {
    return null;
  }

  const known = roomIds.get(rows);

  if (known !== undefined) {
    return known;
  }

  const found = Object.values(rows).find((row) => isNoteToSelf(row, viewerId));
  const id = found?.room.id ?? null;

  roomIds.set(rows, id);

  return id;
}

/**
 * Whether the sidebar offers the note-to-self before it exists: the server leads its placeholders
 * with you while you have no row for it.
 */
export function offersNoteToSelf(state: State): boolean {
  const viewerId = viewerIdOf(state);

  if (viewerId === null || !state.sidebar.placeholderUserIds.includes(viewerId)) {
    return false;
  }

  const id = noteToSelfRoomId(state);
  const row = id === null ? undefined : state.sidebar.rows[id];

  return row === undefined || row.membership.involvement === "invisible";
}

/** Whether `roomId` is your note-to-self, from its detail or its sidebar row. */
export function roomIsNoteToSelf(state: State, roomId: number): boolean {
  const source = state.rooms[roomId]?.detail ?? state.sidebar.rows[roomId];

  return source !== undefined && isNoteToSelf(source, viewerIdOf(state));
}

export function useNoteToSelf(roomId: number): boolean {
  return useStore((state) => roomIsNoteToSelf(state, roomId));
}

export function useNoteToSelfRoomId(): number | null {
  return useStore(noteToSelfRoomId);
}

export function useViewerName(): string | null {
  return useStore(viewerNameOf);
}

/** A room's label with "(you)" after it when it's your note-to-self (search, saved, forward). */
export function useYouLabel(roomId: number, name: string): string {
  return useNoteToSelfRoomId() === roomId ? withYou(name) : name;
}
