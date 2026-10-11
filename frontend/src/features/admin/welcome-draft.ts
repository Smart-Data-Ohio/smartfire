/**
 * The welcome editor's working copy and its checks, the server's rules (crates/api/src/welcome.rs)
 * repeated so a mistake shows beside its field before Save. The server checks again, and alone
 * decides which rooms may be suggested.
 */
import type { GuideResourceEdit } from "../../gen/GuideResourceEdit.ts";
import type { WelcomeRoomEdit } from "../../gen/WelcomeRoomEdit.ts";
import type { WelcomeUpdate } from "../../gen/WelcomeUpdate.ts";

export const MESSAGE_MAX = 1000;

export const ROOMS_MAX = 5;

export const DESCRIPTION_MAX = 100;

export const GUIDE_MAX = 10;

export const TITLE_MAX = 80;

export const BODY_MAX = 1000;

const EMOJI_MAX = 16;

/**
 * A suggested room being edited; `key` keeps its row while rows move. `keep` stands for a saved
 * room this administrator can't see (the server sent no id); it goes back as is until another
 * room is picked.
 */
export interface RoomDraft {
  readonly key: number;
  readonly roomId: number | null;
  readonly keep: string | null;
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
  readonly keep: string | null;
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

export function roomDrafts(rooms: readonly WelcomeRoomEdit[]): RoomDraft[] {
  return rooms.map((room) => ({
    key: draftKey(),
    roomId: room.roomId,
    keep: room.keep,
    emoji: room.emoji ?? "",
    description: room.description,
  }));
}

export function entryDrafts(guide: readonly GuideResourceEdit[]): EntryDraft[] {
  return guide.map((entry) => ({
    key: draftKey(),
    title: entry.title,
    emoji: entry.emoji ?? "",
    kind: entry.roomId === null && entry.keep === null ? "note" : "room",
    roomId: entry.roomId,
    keep: entry.keep,
    body: entry.body ?? "",
  }));
}

export function blankRoom(): RoomDraft {
  return { key: draftKey(), roomId: null, keep: null, emoji: "", description: "" };
}

export function blankEntry(): EntryDraft {
  return {
    key: draftKey(),
    title: "",
    emoji: "",
    kind: "note",
    roomId: null,
    keep: null,
    body: "",
  };
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
      if (room.keep === null) {
        errors.room = "Choose a room.";
      }
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

    if (entry.kind === "room" && entry.roomId === null && entry.keep === null) {
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

/** What the welcome form saves: its message and rooms. Rows without a room are left out. */
export function welcomeUpdateOf(message: string, rooms: readonly RoomDraft[]): WelcomeUpdate {
  return {
    message: message.trim(),
    rooms: rooms.flatMap((room) =>
      room.roomId === null && room.keep === null
        ? []
        : [
            {
              roomId: room.roomId,
              keep: room.roomId === null ? room.keep : null,
              emoji: emojiOrNull(room.emoji),
              description: room.description.trim(),
            },
          ],
    ),
  };
}

/** What the guide form saves: its entries, in order. */
export function guideUpdateOf(entries: readonly EntryDraft[]): WelcomeUpdate {
  return {
    guide: entries.map((entry) => {
      const link = entry.kind === "room";

      return {
        title: entry.title.trim(),
        emoji: emojiOrNull(entry.emoji),
        roomId: link ? entry.roomId : null,
        keep: link && entry.roomId === null ? entry.keep : null,
        body: link ? null : entry.body.trim(),
      };
    }),
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
