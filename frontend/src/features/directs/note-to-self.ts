/**
 * The note-to-self (Slack's "message yourself"): the direct room whose only member is you, for
 * notes, drafts and links. The server names it after you and offers it first among the sidebar's
 * placeholders until it exists; `POST /directs` with nobody else opens it, creating it on first
 * use. Everywhere it shows, it reads as "Your Name (you)".
 */
import { isNoteToSelf, sidebarOffersNoteToSelf } from "../../store/note-to-self.ts";
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

/** What your notes are called when you have no name to show. */
export const NOTES_FALLBACK = "Your notes";

/** What you are called among people when you have no name to show. */
export const YOU_FALLBACK = "You";

/**
 * The first name that says something, trimmed: the shared identity fallback (display name, which
 * is the nickname or else the account name, then the account name), so an empty or blank name
 * never stands alone before "(you)". `null` when there is none.
 */
export function ownName(...names: readonly (string | null | undefined)[]): string | null {
  for (const name of names) {
    const trimmed = name?.trim() ?? "";

    if (trimmed !== "") {
      return trimmed;
    }
  }

  return null;
}

/** "Your Name (you)", as plain text; `fallback` alone when there is no name to show. */
export function withYou(name: string | null | undefined, fallback = NOTES_FALLBACK): string {
  const own = ownName(name);

  return own === null ? fallback : `${own} ${YOU_TAG}`;
}

interface NamedUser {
  readonly name: string;
  readonly accountName?: string;
}

/** Names people by their profiles in `users`, yours with "(you)" after it (switcher, search). */
export function namesWithYou(
  users: Readonly<Record<number, NamedUser>>,
  viewerId: number | null,
): (userId: number) => string | undefined {
  return (userId) => {
    const user = users[userId];

    if (user === undefined || userId !== viewerId) {
      return user?.name;
    }

    return withYou(ownName(user.name, user.accountName), YOU_FALLBACK);
  };
}

export function viewerIdOf(state: State): number | null {
  return state.me?.user.id ?? state.boot?.user.id ?? null;
}

/** Your own name, as the sidebar and headers show it; `null` when every name you have is blank. */
export function viewerNameOf(state: State): string | null {
  const id = viewerIdOf(state);
  const user = id === null ? undefined : state.users[id];

  return ownName(user?.name, user?.accountName, state.me?.user.name, state.me?.user.accountName);
}

/**
 * Your note-to-self's room id, shown or hidden, as the server names it (the sidebar snapshot, and
 * any row or detail marked `noteToSelf`), or `null` before it exists.
 */
export function noteToSelfRoomId(state: State): number | null {
  return state.sidebar.noteToSelfRoomId;
}

/**
 * Whether the sidebar offers the note-to-self before it exists: the server leads its placeholders
 * with you while you have no row for it.
 */
export function offersNoteToSelf(state: State): boolean {
  return sidebarOffersNoteToSelf(state.sidebar, viewerIdOf(state));
}

/** Whether `roomId` is your note-to-self, as the server marks it. */
export function roomIsNoteToSelf(state: State, roomId: number): boolean {
  const source = state.rooms[roomId]?.detail ?? state.sidebar.rows[roomId];

  return (
    state.sidebar.noteToSelfRoomId === roomId || (source !== undefined && isNoteToSelf(source))
  );
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
