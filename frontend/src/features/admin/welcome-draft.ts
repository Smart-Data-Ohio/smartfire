/**
 * The welcome editor's working copy and its checks, the server's rules (crates/api/src/welcome.rs)
 * repeated so a mistake shows beside its field before Save. The server checks again, and alone
 * decides which rooms may be suggested.
 */
import type { GuideResource } from "../../gen/GuideResource.ts";
import type { WelcomeRoom } from "../../gen/WelcomeRoom.ts";
import type { WelcomeSettings } from "../../gen/WelcomeSettings.ts";

export const MESSAGE_MAX = 1000;

export const ROOMS_MAX = 5;

export const DESCRIPTION_MAX = 100;

export const GUIDE_MAX = 10;

export const TITLE_MAX = 80;

export const BODY_MAX = 1000;

const EMOJI_MAX = 16;

/** A suggested room being edited; `key` keeps its row while rows move. */
export interface RoomDraft {
  readonly key: number;
  readonly roomId: number | null;
  readonly emoji: string;
  readonly description: string;
}

/** A guide entry being edited: a room link or a note, whichever `kind` says. */
export interface EntryDraft {
  readonly key: number;
  readonly title: string;
  readonly emoji: string;
  readonly kind: "room" | "note";
  readonly roomId: number | null;
  readonly body: string;
}

export type RowField = "room" | "emoji" | "description" | "title" | "body";

export type RowErrors = Partial<Record<RowField, string>>;

let nextKey = 0;

export function draftKey(): number {
  nextKey += 1;

  return nextKey;
}

/** Characters as the server counts them (code points, not UTF-16 units). */
export function length(text: string): number {
  return [...text].length;
}

export function roomDrafts(rooms: readonly WelcomeRoom[]): RoomDraft[] {
  return rooms.map((room) => ({
    key: draftKey(),
    roomId: room.roomId,
    emoji: room.emoji ?? "",
    description: room.description,
  }));
}

export function entryDrafts(guide: readonly GuideResource[]): EntryDraft[] {
  return guide.map((entry) => ({
    key: draftKey(),
    title: entry.title,
    emoji: entry.emoji ?? "",
    kind: entry.roomId === null ? "note" : "room",
    roomId: entry.roomId,
    body: entry.body ?? "",
  }));
}

export function blankRoom(): RoomDraft {
  return { key: draftKey(), roomId: null, emoji: "", description: "" };
}

export function blankEntry(): EntryDraft {
  return { key: draftKey(), title: "", emoji: "", kind: "note", roomId: null, body: "" };
}

/** A control character other than a line break or tab (multi-line text allows those). */
const CONTROL = /[\p{Cc}]/u;

const CONTROL_EXCEPT_BREAKS = /[^\P{Cc}\n\r\t]/u;

function emojiError(emoji: string): string | undefined {
  const trimmed = emoji.trim();

  if (trimmed === "") {
    return undefined;
  }

  // biome-ignore lint/suspicious/noControlCharactersInRegex: ASCII and whitespace are what a single emoji can't hold
  const plain = /[\u0000-\u007f\s]/u.test(trimmed);

  return plain || length(trimmed) > EMOJI_MAX
    ? "Use a single emoji, or leave it empty."
    : undefined;
}

export function messageError(message: string): string | undefined {
  const trimmed = message.trim();

  if (length(trimmed) > MESSAGE_MAX) {
    return `Keep it to ${MESSAGE_MAX.toLocaleString()} characters or fewer.`;
  }

  return CONTROL_EXCEPT_BREAKS.test(trimmed) ? "Remove the control characters." : undefined;
}

export function roomErrors(rooms: readonly RoomDraft[]): RowErrors[] {
  const seen = new Set<number>();

  return rooms.map((room) => {
    const errors: RowErrors = {};
    const description = room.description.trim();

    if (room.roomId === null) {
      errors.room = "Choose a room.";
    } else if (seen.has(room.roomId)) {
      errors.room = "This room is already suggested.";
    } else {
      seen.add(room.roomId);
    }

    if (length(description) > DESCRIPTION_MAX) {
      errors.description = `Keep it to ${DESCRIPTION_MAX} characters or fewer.`;
    } else if (CONTROL.test(description)) {
      errors.description = "Keep it to one line of text.";
    }

    const emoji = emojiError(room.emoji);

    if (emoji !== undefined) {
      errors.emoji = emoji;
    }

    return errors;
  });
}

export function entryErrors(entries: readonly EntryDraft[]): RowErrors[] {
  return entries.map((entry) => {
    const errors: RowErrors = {};
    const title = entry.title.trim();
    const body = entry.body.trim();

    if (title === "") {
      errors.title = "Give it a title.";
    } else if (length(title) > TITLE_MAX || CONTROL.test(title)) {
      errors.title = `Keep it to one line of ${TITLE_MAX} characters or fewer.`;
    }

    if (entry.kind === "room" && entry.roomId === null) {
      errors.room = "Choose a room.";
    }

    if (entry.kind === "note") {
      if (body === "") {
        errors.body = "Write the note.";
      } else if (length(body) > BODY_MAX) {
        errors.body = `Keep it to ${BODY_MAX.toLocaleString()} characters or fewer.`;
      } else if (CONTROL_EXCEPT_BREAKS.test(body)) {
        errors.body = "Remove the control characters.";
      }
    }

    const emoji = emojiError(entry.emoji);

    if (emoji !== undefined) {
      errors.emoji = emoji;
    }

    return errors;
  });
}

export function hasErrors(rows: readonly RowErrors[]): boolean {
  return rows.some((row) => Object.keys(row).length > 0);
}

function emojiOrNull(emoji: string): string | null {
  const trimmed = emoji.trim();

  return trimmed === "" ? null : trimmed;
}

/** The settings a valid draft saves. Rows without a room are left out (they don't validate). */
export function settingsOf(
  message: string,
  rooms: readonly RoomDraft[],
  entries: readonly EntryDraft[],
): WelcomeSettings {
  return {
    message: message.trim(),
    rooms: rooms.flatMap((room) =>
      room.roomId === null
        ? []
        : [
            {
              roomId: room.roomId,
              emoji: emojiOrNull(room.emoji),
              description: room.description.trim(),
            },
          ],
    ),
    guide: entries.map((entry) => ({
      title: entry.title.trim(),
      emoji: emojiOrNull(entry.emoji),
      roomId: entry.kind === "room" ? entry.roomId : null,
      body: entry.kind === "note" ? entry.body.trim() : null,
    })),
  };
}

/** `items` with the one at `from` moved to `to`. */
export function moved<T>(items: readonly T[], from: number, to: number): T[] {
  const next = [...items];
  const [item] = next.splice(from, 1);

  if (item !== undefined) {
    next.splice(to, 0, item);
  }

  return next;
}
