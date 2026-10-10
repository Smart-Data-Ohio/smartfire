import { describe, expect, it } from "vitest";
import { applyCompletion, findTrigger, roomLink } from "./trigger.ts";

/** `|` marks the caret. */
function at(marked: string) {
  const caret = marked.indexOf("|");
  const text = marked.replace("|", "");

  return findTrigger(text, caret);
}

describe("findTrigger", () => {
  it("finds an @ mention at the start or after whitespace", () => {
    expect(at("@ad|")).toEqual({ kind: "mention", query: "ad", start: 0, end: 3 });
    expect(at("hi @ad|")).toEqual({ kind: "mention", query: "ad", start: 3, end: 6 });
    expect(at("(@ad|")).toMatchObject({ kind: "mention", query: "ad" });
    expect(at("@|")).toMatchObject({ kind: "mention", query: "" });
  });

  it("ignores @ in the middle of a word, like an email address", () => {
    expect(at("ada@exam|")).toBeNull();
  });

  it("keeps a mention open across up to three words, not after a leading space", () => {
    expect(at("@Ada Love|")).toMatchObject({ kind: "mention", query: "Ada Love" });
    expect(at("@Mary Ann Smi|")).toMatchObject({ query: "Mary Ann Smi" });
    expect(at("@one two three four|")).toBeNull();
    expect(at("@ ada|")).toBeNull();
  });

  it("uses the caret, not the end of the text", () => {
    expect(at("@ad| more text")).toMatchObject({ kind: "mention", query: "ad", end: 3 });
    expect(at("@ada done |")).toMatchObject({ kind: "mention", query: "ada done " });
  });

  it("doesn't reopen on an inserted mention token", () => {
    expect(at("@[Ada Lovelace] |")).toBeNull();
    expect(at("<@123>|")).toBeNull();
    expect(at("<@123> |")).toBeNull();
  });

  it("stays on the caret's line", () => {
    expect(at("@ada\nhello|")).toBeNull();
    expect(at("first\n@gr|")).toEqual({ kind: "mention", query: "gr", start: 6, end: 9 });
  });

  it("finds :emoji after two characters, never inside times or words", () => {
    expect(at(":sm|")).toEqual({ kind: "emoji", query: "sm", start: 0, end: 3 });
    expect(at("nice :+1|")).toMatchObject({ kind: "emoji", query: "+1" });
    expect(at(":s|")).toBeNull();
    expect(at("at 12:30|")).toBeNull();
    expect(at("see:th|")).toBeNull();
    expect(at(":smile: |")).toBeNull();
  });

  it("finds a / command only as the first thing in the message", () => {
    expect(at("/|")).toEqual({ kind: "command", query: "", start: 0, end: 1 });
    expect(at("/rem|")).toEqual({ kind: "command", query: "rem", start: 0, end: 4 });
    expect(at("/remind |")).toBeNull();
    expect(at("a /rem|")).toBeNull();
    expect(at(" /rem|")).toBeNull();
    expect(at("//rem|")).toBeNull();
  });

  it("finds #rooms, closing on whitespace", () => {
    expect(at("#gen|")).toEqual({ kind: "room", query: "gen", start: 0, end: 4 });
    expect(at("see #|")).toMatchObject({ kind: "room", query: "" });
    expect(at("# Heading|")).toBeNull();
    expect(at("issue#4|")).toBeNull();
  });

  it("prefers the trigger nearest the caret", () => {
    expect(at("@ada :sm|")).toMatchObject({ kind: "emoji", query: "sm" });
    expect(at("#general @gr|")).toMatchObject({ kind: "mention", query: "gr" });
  });

  it("gives up while text is selected", () => {
    expect(findTrigger("@ada", 1, 4)).toBeNull();
  });
});

describe("applyCompletion", () => {
  it("inserts a stable user ID and keeps it intact when editing surrounding text", () => {
    expect(
      applyCompletion(
        "<@123> and @ad tomorrow",
        { kind: "mention", query: "ad", start: 11, end: 14 },
        "<@456>",
      ),
    ).toEqual({ value: "<@123> and <@456> tomorrow", caret: 18 });
  });
  it("replaces the trigger and query, adds a space and puts the caret after it", () => {
    const trigger = { kind: "mention", query: "ad", start: 3, end: 6 } as const;

    expect(applyCompletion("hi @ad", trigger, "@[Ada Lovelace]")).toEqual({
      value: "hi @[Ada Lovelace] ",
      caret: 19,
    });
  });

  it("keeps the text after the caret and doesn't double a space", () => {
    const trigger = { kind: "emoji", query: "sm", start: 0, end: 3 } as const;

    expect(applyCompletion(":sm there", trigger, ":smile:")).toEqual({
      value: ":smile: there",
      caret: 8,
    });
  });

  it("can skip the trailing space", () => {
    const trigger = { kind: "command", query: "sh", start: 0, end: 3 } as const;

    expect(applyCompletion("/sh", trigger, "/shrug", false)).toEqual({ value: "/shrug", caret: 6 });
  });
});

describe("roomLink", () => {
  it("links to the room in place, dropping brackets from the label", () => {
    expect(roomLink("general", 12)).toBe("[#general](/rooms/12)");
    expect(roomLink("odd [name]", 3)).toBe("[#odd name](/rooms/3)");
  });
});
