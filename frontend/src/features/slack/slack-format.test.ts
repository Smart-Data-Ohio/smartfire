import { describe, expect, it } from "vitest";
import type { SlackPlan } from "../../gen/SlackPlan.ts";
import {
  activeRunSentence,
  adminConnectionSentence,
  countLines,
  importBody,
  keyed,
  modeLabel,
  optional,
  parseRunSearch,
  peopleLine,
  personalConnectionSentence,
} from "./slack-format.ts";

const conversation = (id: string) => ({
  id,
  name: id.toLowerCase(),
  kind: "Public channel",
  archived: false,
  members: 1,
  messages: 1,
  threads: 0,
});

const PLAN: SlackPlan = {
  runId: 7,
  conversations: [
    { conversation: conversation("C1"), target: "new" },
    { conversation: conversation("C2"), target: "4" },
    { conversation: conversation("C3"), target: "skip" },
  ],
  rooms: [{ id: 4, name: "General" }],
  samples: [],
  defaultOldest: "2026-09-22",
};

describe("the Slack import words", () => {
  it("say runs as the classic pages do", () => {
    expect(modeLabel("dry_run")).toBe("dry run");
    expect(
      activeRunSentence({ id: 3, kind: "workspace", mode: "dry_run", status: "running" }),
    ).toBe("A workspace dry run is running.");
    expect(peopleLine({ total: 9, matched: 6, placeholders: 2, deactivated: 1, bots: 0 })).toBe(
      "9 found (6 matched, 2 placeholders, 1 deactivated, 0 bots)",
    );
    expect(
      countLines({
        roomsCreated: 2,
        roomsMerged: 1,
        messages: 300,
        replies: 40,
        threads: 8,
        reactions: 12,
        pins: 1,
        filesLinked: 3,
        skipped: 0,
      }),
    ).toEqual({
      rooms: "2 created, 1 merged",
      messages: "300 messages, 40 replies in 8 threads",
      extras: "12 reactions, 1 pins, 3 files linked, 0 skipped",
    });
  });

  it("describe a connection for the setup page and the personal page", () => {
    expect(adminConnectionSentence({ state: "connected" }, "Acme")).toBe(
      "Connected to the Acme Slack workspace. Reconnect to refresh the grant.",
    );
    expect(adminConnectionSentence({ state: "rejected", reason: "token_revoked" }, null)).toBe(
      "Slack rejected the connection (token_revoked). Reconnect below.",
    );
    expect(personalConnectionSentence({ state: "rejected", reason: null })).toBe(
      "Slack rejected the connection. Reconnect below.",
    );
    expect(personalConnectionSentence({ state: "none" })).toContain("Connect with your Slack");
  });
});

describe("the plan form", () => {
  it("posts the checked conversations in plan order and every row's target", () => {
    expect(
      importBody(PLAN, new Set(["C3", "C1"]), { C1: "4" }, "test", {
        oldest: " 2026-09-22 ",
        latest: "",
      }),
    ).toEqual({
      conversationIds: ["C1", "C3"],
      roomTargets: { C1: "4", C2: "4", C3: "skip" },
      preset: "test",
      oldest: "2026-09-22",
      latest: null,
    });
  });

  it("treats blanks as unset", () => {
    expect(optional("  ")).toBeNull();
    expect(optional(" x ")).toBe("x");
  });
});

describe("row keys", () => {
  it("follow the content, numbering repeats, and hold as rows are added at the end", () => {
    const first = keyed(["a", "b", "a"], (row) => row).map((each) => each.key);
    const grown = keyed(["a", "b", "a", "a"], (row) => row).map((each) => each.key);

    expect(new Set(first).size).toBe(3);
    expect(grown.slice(0, 3)).toEqual(first);
    expect(new Set(grown).size).toBe(4);
  });
});

describe("a run page's query", () => {
  it("reads a later page from a classic link, and the first page otherwise", () => {
    expect(parseRunSearch({ page: 3 })).toEqual({ page: 3 });
    expect(parseRunSearch({ page: "2" })).toEqual({ page: 2 });
    expect(parseRunSearch({ page: 1 })).toEqual({ page: undefined });
    expect(parseRunSearch({ page: "x" })).toEqual({ page: undefined });
    expect(parseRunSearch({})).toEqual({ page: undefined });
  });
});
