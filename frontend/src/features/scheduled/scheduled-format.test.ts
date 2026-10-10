import { describe, expect, it } from "vitest";
import type { ScheduledMessage } from "../../gen/ScheduledMessage.ts";
import { outcomeLabel, scheduledExcerpt, scheduledSection } from "./scheduled-format.ts";

function scheduled(overrides: Partial<ScheduledMessage>): ScheduledMessage {
  return {
    id: 1,
    roomId: 12,
    threadId: null,
    replyToMessageId: null,
    replyTarget: null,
    attachments: [],
    markdownSource: "Hello",
    excerpt: "Hello",
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

describe("scheduledExcerpt", () => {
  it("drops emphasis markers and folds whitespace", () => {
    const source = "**Ship** it\n\n> _today_";

    expect(scheduledExcerpt(scheduled({ markdownSource: source, excerpt: source }))).toBe(
      "Ship it today",
    );
  });

  // The server renders the source and redacts it (`markdown::redacted_excerpt`); these pairs
  // come from crates/richtext/tests/spoilers.rs. The client never reads the source.
  const REDACTED: readonly (readonly [string, string])[] = [
    ["see ||the ending|| now", "see spoiler now"],
    ['[||x||](https://example.com/a\\)b "Alice dies") after', "spoiler after"],
    ['[||x||](<https://example.com/a)b> "Alice dies") after', "spoiler after"],
    [
      'read [||x||][r] now\n\n[r]:\n  https://example.com/alice-dies\n  "Alice dies"',
      "read spoiler now",
    ],
    ['> [||x||][r]\n>\n> [r]: https://example.com/alice-dies "Alice dies"', "spoiler"],
    ["[a [||x||] b](https://example.com/alice-dies)", "a [spoiler] b"],
  ];

  it.each(REDACTED)("shows the server's excerpt of %j, never the source", (source, excerpt) => {
    const shown = scheduledExcerpt(scheduled({ markdownSource: source, excerpt }));

    expect(shown).toBe(excerpt);
    expect(shown).not.toMatch(/Alice|alice-dies|example\.com/);
  });
});
