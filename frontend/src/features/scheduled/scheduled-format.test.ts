import { describe, expect, it } from "vitest";
import type { ScheduledMessage } from "../../gen/ScheduledMessage.ts";
import { markdownExcerpt, outcomeLabel, scheduledSection } from "./scheduled-format.ts";

function scheduled(overrides: Partial<ScheduledMessage>): ScheduledMessage {
  return {
    id: 1,
    roomId: 12,
    threadId: null,
    replyToMessageId: null,
    markdownSource: "Hello",
    sendAt: "2026-10-07T13:00:00.000Z",
    state: "pending",
    sendable: true,
    sentAt: null,
    sentMessageId: null,
    droppedAt: null,
    dropReason: null,
    createdAt: "2026-10-06T12:00:00.000Z",
    ...overrides,
  };
}

describe("scheduledSection", () => {
  it("splits pending into upcoming and stranded, the rest into past", () => {
    expect(scheduledSection(scheduled({}))).toBe("upcoming");
    expect(scheduledSection(scheduled({ state: "sending" }))).toBe("upcoming");
    expect(scheduledSection(scheduled({ sendable: false }))).toBe("stranded");
    expect(scheduledSection(scheduled({ state: "sent", sendable: false }))).toBe("past");
    expect(scheduledSection(scheduled({ state: "dropped", sendable: false }))).toBe("past");
  });
});

describe("outcomeLabel", () => {
  it("says why a dropped message wasn't sent", () => {
    const now = Date.parse("2026-10-06T12:00:00.000Z");

    expect(
      outcomeLabel(scheduled({ state: "dropped", dropReason: "its room was deleted" }), now),
    ).toBe("Not sent: its room was deleted");
    expect(outcomeLabel(scheduled({ state: "dropped" }), now)).toBe(
      "Not sent: you lost access to the conversation",
    );
    expect(outcomeLabel(scheduled({ state: "sent" }), now)).toBe("Sent");
  });
});

describe("markdownExcerpt", () => {
  it("drops emphasis markers and folds whitespace", () => {
    expect(markdownExcerpt("**Ship** it\n\n> _today_")).toBe("Ship it today");
  });

  it("replaces a spoiler with the word, and hides more rather than less", () => {
    expect(markdownExcerpt("see ||the ending|| now")).toBe("see spoiler now");
    expect(markdownExcerpt("||outer ||SECRET|| tail||")).toBe("spoiler");
    expect(markdownExcerpt("\\`||SECRET||\\` tail")).toBe("\\spoiler\\ tail");
    expect(markdownExcerpt("use `||the ending||` here")).toBe("use spoiler here");
  });
});
