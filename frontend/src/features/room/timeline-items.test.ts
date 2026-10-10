import { describe, expect, it } from "vitest";
import type { MessageDTO, PendingMessage, Timeline } from "../../store/model.ts";
import {
  addPending,
  applyEvents,
  applyPage,
  receiveMessage,
  setPageReplacing,
} from "../../store/reducers.ts";
import { emptyTimeline, initialState, type State } from "../../store/state.ts";
import { firstMessageKey, prepended, type TimelineItem, timelineItems } from "./timeline-items.ts";

function message(id: number, creatorId: number, createdAt: string, systemNote = false): MessageDTO {
  return {
    id,
    roomId: 1,
    threadId: null,
    creatorId,
    clientMessageId: `client-${id}`,
    sound: null,
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
    poll: null,
    cards: [],
    cardsAsOf: createdAt,
    steps: [],
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
  introFirst = false,
): TimelineItem[] {
  return timelineItems({
    introFirst,
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
      replyToMessageId: null,
      replyNotifyAuthor: null,
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

    expect(
      summary(layout([message(1, 7, local(6, 9, 0))], { after: 1, arrived: [] }, [pending])),
    ).toEqual(["intro", "day:Today", "1*", "p"]);
  });

  it("keys a confirmed message like its pending row", () => {
    const [, , row] = layout([message(1, 7, local(6, 9, 0))]);

    expect(row?.key).toBe("c-client-1");
  });

  it.each(["POST response", "broadcast"])(
    "keeps a sent row visible when its %s arrives before the latest page",
    (first) => {
      const history = message(1, 7, local(6, 9, 0));
      const confirmed = message(9, 7, local(6, 9, 2));

      const pending: PendingMessage = {
        clientMessageId: confirmed.clientMessageId,
        roomId: 1,
        threadId: null,
        attachmentSignedId: null,
        attachment: null,
        replyToMessageId: null,
        replyNotifyAuthor: null,
        creatorId: 7,
        markdownSource: "9",
        createdAt: confirmed.createdAt,
        state: "sending",
        error: null,
      };

      const page = { messages: [history], users: [], before: null, after: 1, saved: [] };

      const sent = addPending(
        setPageReplacing(applyPage(initialState, 1, page, "replace"), 1),
        pending,
      );

      const rows = (state: State) =>
        timelineItems({
          timeline: state.timelines[1] ?? emptyTimeline,
          messages: state.messages,
          pending: Object.values(state.pending),
          now,
        });

      const broadcast = (state: State) =>
        applyEvents(
          state,
          [{ seq: 1, topic: "room:1", type: "message.created", data: confirmed }],
          0,
        );

      expect(summary(rows(sent))).toEqual(["intro", "day:Today", "1*", "p"]);

      const reconciled =
        first === "POST response" ? receiveMessage(sent, confirmed) : broadcast(sent);

      expect(summary(rows(reconciled))).toEqual(["intro", "day:Today", "1*", "9"]);
      expect(rows(reconciled).at(-1)?.key).toBe(rows(sent).at(-1)?.key);
      expect(reconciled.pending).toEqual({});

      const echoed =
        first === "POST response" ? broadcast(reconciled) : receiveMessage(reconciled, confirmed);

      expect(summary(rows(echoed))).toEqual(["intro", "day:Today", "1*", "9"]);
      // The GET was read before this send, so reconciliation must also survive a stale page.
      const landed = applyPage(echoed, 1, { ...page, after: null }, "replace");

      expect(summary(rows(landed))).toEqual(["intro", "day:Today", "1*", "9"]);
    },
  );
});

describe("prepended", () => {
  const today = [message(5, 1, local(6, 9, 0)), message(6, 1, local(6, 9, 1))];
  const older = [message(3, 1, local(6, 8, 0)), message(4, 1, local(6, 8, 1))];

  const edges = (items: readonly TimelineItem[]) => ({
    first: items[0]?.key ?? null,
    firstMessage: firstMessageKey(items),
  });

  it("holds the view while an older page's spinner comes and when the page replaces it", () => {
    const shown = layout(today, { before: 4 });
    const loading = layout(today, { before: 4, loadingOlder: true });
    // Same day: the day divider keeps its key, and the spinner that was on top is gone.
    const landed = layout([...older, ...today], { before: 2 });

    expect(prepended(loading, edges(shown))).toBe(true);
    expect(prepended(landed, edges(loading))).toBe(true);
  });

  it("leaves appends and fresh windows alone", () => {
    const shown = layout(today, { before: 4 });
    const appended = layout([...today, message(7, 1, local(6, 9, 2))], { before: 4 });

    expect(prepended(appended, edges(shown))).toBe(false);
    expect(prepended(layout(older, { before: 2 }), edges(shown))).toBe(false);
  });

  it("keeps an intro that leads first above a window short of the start", () => {
    const list = [message(1, 7, local(6, 9, 0))];

    expect(summary(layout(list, { before: 1 }, [], true))).toEqual([
      "intro",
      "earlier",
      "day:Today",
      "1*",
    ]);
    expect(summary(layout(list, { before: 1, loadingOlder: true }, [], true))).toEqual([
      "intro",
      "loading",
      "day:Today",
      "1*",
    ]);
    expect(summary(layout(list, {}, [], true))).toEqual(["intro", "day:Today", "1*"]);
  });
});
