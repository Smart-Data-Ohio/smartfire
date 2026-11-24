import { describe, expect, it } from "vitest";
import type { SlackConversation } from "../../gen/SlackConversation.ts";
import type { SlackPlan } from "../../gen/SlackPlan.ts";
import {
  activeRunSentence,
  adminConnectionSentence,
  countLines,
  coversPastYear,
  importBody,
  inactive,
  inactiveHint,
  inactiveReason,
  keyed,
  modeLabel,
  optional,
  parseRunSearch,
  peopleLine,
  personalConnectionSentence,
} from "./slack-format.ts";
import { MAX_ISSUE_PAGES } from "./slack-pages.ts";

const conversation = (id: string) => ({
  id,
  name: id.toLowerCase(),
  kind: "Public channel",
  archived: false,
  members: 1,
  messages: 1,
  threads: 0,
  lastMessageAt: null,
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
  scannedFrom: null,
  scannedUntil: null,
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
    expect(parseRunSearch({ page: "1000000" })).toEqual({ page: MAX_ISSUE_PAGES });
  });
});

describe("inactive", () => {
  const NOW = Date.parse("2026-11-24T12:00:00Z");

  const channel = (changes: Partial<SlackConversation>): SlackConversation => ({
    ...conversation("C9"),
    members: 4,
    messages: 30,
    lastMessageAt: "2026-06-01T09:00:00.000Z",
    ...changes,
  });

  it("keeps a channel with members and a message in the past year", () => {
    expect(inactive(channel({}), NOW, true)).toBe(false);
    expect(inactiveReason(channel({}), NOW, true)).toBeNull();
  });

  it("leaves out a channel with no members or no messages", () => {
    expect(inactiveReason(channel({ members: 0 }), NOW, true)).toBe("No members");
    expect(inactiveReason(channel({ messages: 0, lastMessageAt: null }), NOW, true)).toBe(
      "No messages",
    );
  });

  it("leaves out a channel whose last message is over a year old", () => {
    expect(inactiveReason(channel({ lastMessageAt: "2025-11-20T09:00:00.000Z" }), NOW, true)).toBe(
      "No messages in the past year",
    );
    expect(inactive(channel({ lastMessageAt: "2025-11-30T09:00:00.000Z" }), NOW, true)).toBe(false);
  });

  it("counts an older dry run without last-message dates as active", () => {
    expect(inactive(channel({ lastMessageAt: null }), NOW, true)).toBe(false);
  });

  it("trusts only the member count when the dry run skipped part of the year", () => {
    const quiet = channel({ messages: 0, lastMessageAt: null });

    expect(inactive(quiet, NOW, false)).toBe(false);
    expect(inactive(channel({ lastMessageAt: "2024-01-01T00:00:00.000Z" }), NOW, false)).toBe(
      false,
    );
    expect(inactiveReason(channel({ members: 0 }), NOW, false)).toBe("No members");
  });
});

describe("coversPastYear", () => {
  const NOW = Date.parse("2026-11-24T12:00:00Z");

  it("needs no latest bound and no oldest bound inside the year", () => {
    expect(coversPastYear({ scannedFrom: null, scannedUntil: null }, NOW)).toBe(true);
    expect(
      coversPastYear({ scannedFrom: "2025-01-01T00:00:00.000Z", scannedUntil: null }, NOW),
    ).toBe(true);
    expect(
      coversPastYear({ scannedFrom: "2026-11-10T00:00:00.000Z", scannedUntil: null }, NOW),
    ).toBe(false);
    expect(
      coversPastYear({ scannedFrom: null, scannedUntil: "2025-05-01T00:00:00.000Z" }, NOW),
    ).toBe(false);
  });
});

describe("inactiveHint", () => {
  it("says how many channels start unchecked and why", () => {
    expect(inactiveHint(0, true)).toBeNull();
    expect(inactiveHint(1, true)).toBe(
      "1 channel starts unchecked: no members, or no messages in the past year. Check one to import it anyway.",
    );
    expect(inactiveHint(3, false)).toBe(
      "3 channels with no members start unchecked. This dry run was limited to a date range, so it can't tell which channels went quiet. Check one to import it anyway.",
    );
    expect(inactiveHint(0, false)).toBe(
      "This dry run was limited to a date range, so it can't tell which channels went quiet.",
    );
  });
});
