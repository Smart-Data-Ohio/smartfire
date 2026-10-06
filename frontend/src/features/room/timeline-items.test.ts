import { describe, expect, it } from "vitest";
import type { MessageDTO, PendingMessage, Timeline } from "../../store/model.ts";
import { emptyTimeline } from "../../store/state.ts";
import { type TimelineItem, timelineItems } from "./timeline-items.ts";

function message(id: number, creatorId: number, createdAt: string, systemNote = false): MessageDTO {
  return {
    id,
    roomId: 1,
    threadId: null,
    creatorId,
    clientMessageId: `client-${id}`,
    bodyHtml: `<p>${id}</p>`,
    markdownSource: `${id}`,
    systemNote,
    action: false,
    streaming: false,
    embedsSuppressed: false,
    replyToMessageId: null,
    forwardedFromMessageId: null,
    forwardedAt: null,
    forwardNote: null,
    editedAt: null,
    attachment: null,
    reactions: [],
    boosts: [],
    pinned: false,
    thread: null,
    createdAt,
    updatedAt: createdAt,
  };
}

function local(day: number, hour: number, minute: number): string {
  return new Date(2026, 9, day, hour, minute).toISOString();
}

const now = new Date(2026, 9, 6, 18, 0).getTime();

function layout(
  list: readonly MessageDTO[],
  timeline: Partial<Timeline> = {},
  pending: readonly PendingMessage[] = [],
): TimelineItem[] {
  return timelineItems({
    timeline: {
      ...emptyTimeline,
      status: "ready",
      ids: list.map((entry) => entry.id),
      ...timeline,
    },
    messages: Object.fromEntries(list.map((entry) => [entry.id, entry])),
    pending,
    now,
  });
}

function summary(items: readonly TimelineItem[]): string[] {
  return items.map((item) => {
    switch (item.kind) {
      case "message":
        return `${item.message.id}${item.groupStart ? "*" : ""}`;
      case "pending":
        return `p${item.groupStart ? "*" : ""}`;
      case "day":
        return `day:${item.label}`;
      default:
        return item.kind;
    }
  });
}

describe("timelineItems", () => {
  it("groups one author's messages within five minutes and breaks on a new day", () => {
    const items = layout([
      message(1, 7, local(5, 9, 0)),
      message(2, 7, local(5, 9, 4)),
      message(3, 7, local(5, 9, 10)),
      message(4, 8, local(5, 9, 11)),
      message(5, 8, local(6, 9, 12)),
    ]);

    expect(summary(items)).toEqual([
      "intro",
      "day:Yesterday",
      "1*",
      "2",
      "3*",
      "4*",
      "day:Today",
      "5*",
    ]);
  });

  it("puts the unread divider before the first unread message and restarts the group there", () => {
    const items = layout([message(1, 7, local(6, 9, 0)), message(2, 7, local(6, 9, 1))], {
      unreadFromId: 2,
      unreadCount: 1,
      before: 99,
    });

    expect(summary(items)).toEqual(["day:Today", "1*", "unread", "2*"]);
  });

  it("never folds a message into a system note", () => {
    const items = layout([message(1, 7, local(6, 9, 0), true), message(2, 7, local(6, 9, 1))]);

    expect(summary(items)).toEqual(["intro", "day:Today", "1*", "2*"]);
  });

  it("shows loading rows at the edges that are fetching", () => {
    const items = layout([message(1, 7, local(6, 9, 0))], {
      before: 1,
      after: 1,
      loadingOlder: true,
      loadingNewer: true,
    });

    expect(summary(items)).toEqual(["loading", "day:Today", "1*", "loading"]);
  });

  it("appends pending sends at the present, grouped with the author's last message", () => {
    const pending: PendingMessage = {
      clientMessageId: "pending-1",
      roomId: 1,
      threadId: null,
      attachmentSignedId: null,
      attachment: null,
      creatorId: 7,
      markdownSource: "hi",
      createdAt: local(6, 9, 2),
      state: "sending",
      error: null,
    };

    expect(summary(layout([message(1, 7, local(6, 9, 0))], {}, [pending]))).toEqual([
      "intro",
      "day:Today",
      "1*",
      "p",
    ]);

    expect(summary(layout([message(1, 7, local(6, 9, 0))], { after: 1 }, [pending]))).toEqual([
      "intro",
      "day:Today",
      "1*",
    ]);
  });

  it("keys a confirmed message like its pending row", () => {
    const [, , row] = layout([message(1, 7, local(6, 9, 0))]);

    expect(row?.key).toBe("c-client-1");
  });
});
