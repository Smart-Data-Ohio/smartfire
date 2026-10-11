import { describe, expect, it } from "vitest";
import {
  blankEntry,
  blankRoom,
  type EntryDraft,
  entryErrors,
  hasErrors,
  messageError,
  moved,
  type RoomDraft,
  roomErrors,
  settingsOf,
} from "./welcome-draft.ts";

const room = (patch: Partial<RoomDraft>): RoomDraft => ({ ...blankRoom(), roomId: 1, ...patch });

const entry = (patch: Partial<EntryDraft>): EntryDraft => ({
  ...blankEntry(),
  title: "Hours",
  body: "Nine to five",
  ...patch,
});

describe("the welcome draft", () => {
  it("checks the message as the server does, counting code points", () => {
    expect(messageError("  Hello  ")).toBeUndefined();
    expect(messageError("😀".repeat(1000))).toBeUndefined();
    expect(messageError("x".repeat(1001))).toMatch(/1,000 characters/);
    expect(messageError("line\nbreak\ttab")).toBeUndefined();
    expect(messageError("bell\u0007")).toMatch(/control/);
  });

  it("checks suggested rooms: one each, a short one-line description and a single emoji", () => {
    const errors = roomErrors([
      room({ roomId: 1, emoji: "👋" }),
      room({ roomId: 1 }),
      room({ roomId: null }),
      room({ roomId: 2, description: "x".repeat(101) }),
      room({ roomId: 3, description: "two\nlines" }),
      room({ roomId: 4, emoji: "wave" }),
      room({ roomId: 5, emoji: "👨‍👩‍👧" }),
    ]);

    expect(errors).toEqual([
      {},
      { room: "This room is already suggested." },
      { room: "Choose a room." },
      { description: "Keep it to 100 characters or fewer." },
      { description: "Keep it to one line of text." },
      { emoji: "Use a single emoji, or leave it empty." },
      {},
    ]);
    expect(hasErrors(errors)).toBe(true);
    expect(hasErrors(roomErrors([room({})]))).toBe(false);
  });

  it("checks guide entries: a title, and a room or a note as the kind says", () => {
    expect(
      entryErrors([
        entry({}),
        entry({ title: "  " }),
        entry({ title: "x".repeat(81) }),
        entry({ kind: "room", roomId: null }),
        entry({ kind: "note", body: " " }),
        entry({ kind: "note", body: "x".repeat(1001) }),
        entry({ kind: "room", roomId: 3, body: "ignored" }),
      ]),
    ).toEqual([
      {},
      { title: "Give it a title." },
      { title: "Keep it to one line of 80 characters or fewer." },
      { room: "Choose a room." },
      { body: "Write the note." },
      { body: "Keep it to 1,000 characters or fewer." },
      {},
    ]);
  });

  it("saves trimmed settings: blank emoji as null, and only what each entry's kind holds", () => {
    expect(
      settingsOf(
        "  Hi  ",
        [room({ roomId: 4, emoji: " 👋 ", description: " Say hello " }), room({ roomId: null })],
        [
          entry({ emoji: " ", body: " Nine to five " }),
          entry({ title: "Ship", kind: "room", roomId: 3, body: "stale note" }),
        ],
      ),
    ).toEqual({
      message: "Hi",
      rooms: [{ roomId: 4, emoji: "👋", description: "Say hello" }],
      guide: [
        { title: "Hours", emoji: null, roomId: null, body: "Nine to five" },
        { title: "Ship", emoji: null, roomId: 3, body: null },
      ],
    });
  });

  it("moves a row up or down", () => {
    expect(moved(["a", "b", "c"], 2, 0)).toEqual(["c", "a", "b"]);
    expect(moved(["a", "b", "c"], 0, 1)).toEqual(["b", "a", "c"]);
  });
});
