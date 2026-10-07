import { describe, expect, it } from "vitest";
import { messageFixture, pageFixture } from "../api/testing.ts";
import type { AgentStep } from "../gen/AgentStep.ts";
import type { MessageCard } from "../gen/MessageCard.ts";
import type { Poll } from "../gen/Poll.ts";
import {
  applyBallot,
  githubKey,
  needsFetch,
  PREVIEW_TTL_MS,
  pollClosed,
  pollView,
  previewLoaded,
  previewLoading,
  reconcileMessage,
  setPendingVote,
  settleVote,
  viewerChoice,
  visibleCards,
  withResponse,
} from "./cards.ts";
import type { MessageDTO, SyncEvent } from "./model.ts";
import { applyEvents, applyPage, receiveMessage, updateMessage } from "./reducers.ts";
import { initialState, type State } from "./state.ts";

const ROOM = 5;

const ME = 7;

const at = (second: number) => new Date(Date.UTC(2026, 9, 6, 12, 0, second)).toISOString();

function poll(asOf: string, votes: readonly [number, number], change: Partial<Poll> = {}): Poll {
  return {
    id: 40,
    messageId: 1,
    asOf,
    multiple: false,
    anonymous: false,
    closesAt: null,
    closedAt: null,
    closed: false,
    totalVotes: votes[0] + votes[1],
    options: [
      { id: 401, label: "Tea", votes: votes[0], voterIds: votes[0] > 0 ? [3] : [] },
      { id: 402, label: "Coffee", votes: votes[1], voterIds: votes[1] > 0 ? [4] : [] },
    ],
    ...change,
  };
}

const github = (pullRequestId: number): MessageCard => ({
  kind: "github",
  data: {
    pullRequestId,
    owner: "acme",
    repo: "app",
    number: pullRequestId,
    url: `https://github.com/acme/app/pull/${pullRequestId}`,
  },
});

function seeded(message: MessageDTO): State {
  return applyPage(initialState, ROOM, pageFixture([message]), "replace");
}

function events(state: State, list: readonly SyncEvent[]): State {
  return applyEvents(state, list, Date.parse(at(59)));
}

describe("reconcileMessage", () => {
  const held = messageFixture(1, ROOM, { poll: poll(at(10), [1, 0]), cardsAsOf: at(10) });

  it("keeps the poll with the later asOf whichever copy is newer by updatedAt", () => {
    const newerBody = { ...held, updatedAt: at(50), poll: poll(at(5), [0, 0]) };
    const merged = reconcileMessage(held, newerBody, false);

    expect(merged.updatedAt).toBe(at(50));
    expect(merged.poll?.asOf).toBe(at(10));
  });

  it("takes the incoming poll and cards on an asOf tie (the later arrival)", () => {
    const incoming = { ...held, poll: poll(at(10), [0, 1]), cards: [github(9)] };
    const merged = reconcileMessage(held, incoming, false);

    expect(merged.poll?.options[1]?.votes).toBe(1);
    expect(merged.cards).toEqual([github(9)]);
  });

  it("keeps the held cards when the incoming ones are older", () => {
    const withCards = { ...held, cards: [github(9)], cardsAsOf: at(20) };
    const stale = { ...held, updatedAt: at(30), cards: [], cardsAsOf: at(15) };
    const merged = reconcileMessage(withCards, stale, false);

    expect(merged.updatedAt).toBe(at(30));
    expect(merged.cards).toEqual([github(9)]);
    expect(merged.cardsAsOf).toBe(at(20));
  });

  it("never drops a poll for a copy without one", () => {
    expect(reconcileMessage(held, { ...held, poll: null }, true).poll).toBe(held.poll);
  });

  it("returns the held message itself when nothing changes", () => {
    expect(reconcileMessage(held, { ...held }, false)).toBe(held);
  });

  it("merges agent steps from both copies while taking the newer poll", () => {
    const step = (id: number, updated: number, status: AgentStep["status"]): AgentStep => ({
      id,
      messageId: held.id,
      threadId: null,
      name: `Step ${id}`,
      status,
      inputSummary: null,
      outputSummary: null,
      durationMs: null,
      position: id,
      createdAt: at(0),
      updatedAt: at(updated),
    });

    const withSteps = { ...held, steps: [step(1, 20, "done"), step(2, 20, "running")] };

    const newerBody = {
      ...held,
      updatedAt: at(50),
      poll: poll(at(30), [2, 0]),
      steps: [step(1, 5, "running")],
    };

    const merged = reconcileMessage(withSteps, newerBody, false);

    expect(merged.updatedAt).toBe(at(50));
    expect(merged.poll?.asOf).toBe(at(30));
    expect(merged.steps.map((each) => [each.id, each.status])).toEqual([
      [1, "done"],
      [2, "running"],
    ]);
  });
});

describe("poll and card events", () => {
  const message = messageFixture(1, ROOM, { poll: poll(at(10), [1, 0]), cardsAsOf: at(10) });

  it("applies poll.updated unless older, and on a tie keeps the later arrival", () => {
    const update = (asOf: string, votes: readonly [number, number]): SyncEvent => ({
      type: "poll.updated",
      seq: 1,
      topic: `room:${ROOM}`,
      data: { roomId: ROOM, threadId: null, poll: poll(asOf, votes) },
    });

    const stale = events(seeded(message), [update(at(5), [9, 9])]);

    expect(stale.messages[1]?.poll?.totalVotes).toBe(1);

    const tie = events(seeded(message), [update(at(10), [1, 1])]);

    expect(tie.messages[1]?.poll?.totalVotes).toBe(2);

    const newer = events(tie, [update(at(20), [2, 1]), update(at(15), [0, 0])]);

    expect(newer.messages[1]?.poll?.totalVotes).toBe(3);
    expect(newer.messages[1]?.updatedAt).toBe(message.updatedAt);
  });

  it("ignores poll.updated for a message it doesn't hold", () => {
    const state = seeded(message);

    const next = events(state, [
      {
        type: "poll.updated",
        seq: 1,
        topic: `room:${ROOM}`,
        data: { roomId: ROOM, threadId: null, poll: { ...poll(at(30), [0, 0]), messageId: 99 } },
      },
    ]);

    expect(next).toBe(state);
  });

  it("applies poll.ballot by asOf, the later arrival winning a tie", () => {
    const ballot = (asOf: string, myOptionIds: number[]): SyncEvent => ({
      type: "poll.ballot",
      seq: 1,
      topic: "user",
      data: { pollId: 40, messageId: 1, roomId: ROOM, threadId: null, myOptionIds, asOf },
    });

    const state = events(seeded(message), [ballot(at(20), [401])]);

    expect(state.cards.ballots[40]?.myOptionIds).toEqual([401]);
    expect(events(state, [ballot(at(10), [402])]).cards.ballots[40]?.myOptionIds).toEqual([401]);
    expect(events(state, [ballot(at(20), [])]).cards.ballots[40]?.myOptionIds).toEqual([]);
  });

  it("replaces cards on message.cards unless older, and drops the previews they point at", () => {
    const withPr = seeded({ ...message, cards: [github(9)] });
    const key = githubKey(ROOM, 9, { messageId: 1 });

    const cached = previewLoaded(withPr, "github", key, 9, { state: "hidden" }, Date.parse(at(11)));

    const cardsEvent = (asOf: string, cards: MessageCard[]): SyncEvent => ({
      type: "message.cards",
      seq: 1,
      topic: `room:${ROOM}`,
      data: { messageId: 1, roomId: ROOM, threadId: null, cards, asOf },
    });

    const stale = events(cached, [cardsEvent(at(5), [])]);

    expect(stale.messages[1]?.cards).toEqual([github(9)]);
    expect(stale.cards.previews.github[key]).toBeDefined();

    const tie = events(cached, [cardsEvent(at(10), [github(9), github(10)])]);

    expect(tie.messages[1]?.cards).toHaveLength(2);
    expect(tie.messages[1]?.cardsAsOf).toBe(at(10));
    expect(tie.cards.previews.github[key]).toBeUndefined();
  });

  it("keeps a newer poll and cards when an older page or edit lands", () => {
    const state = events(seeded(message), [
      {
        type: "poll.updated",
        seq: 1,
        topic: `room:${ROOM}`,
        data: { roomId: ROOM, threadId: null, poll: poll(at(30), [3, 3]) },
      },
    ]);

    const refreshed = applyPage(state, ROOM, pageFixture([message]), "refresh");

    expect(refreshed.messages[1]?.poll?.totalVotes).toBe(6);

    const edited = updateMessage(refreshed, {
      ...message,
      updatedAt: at(40),
      markdownSource: "Edited",
    });

    expect(edited.messages[1]?.markdownSource).toBe("Edited");
    expect(edited.messages[1]?.poll?.totalVotes).toBe(6);

    const echoed = receiveMessage(edited, { ...message, poll: poll(at(45), [4, 3]) });

    expect(echoed.messages[1]?.markdownSource).toBe("Edited");
    expect(echoed.messages[1]?.poll?.totalVotes).toBe(7);
  });
});

describe("reading polls", () => {
  it("works out the viewer's choice from voters, or the ballot when it's as new", () => {
    const open = poll(at(10), [1, 1], {
      options: [
        { id: 401, label: "Tea", votes: 1, voterIds: [ME] },
        { id: 402, label: "Coffee", votes: 1, voterIds: [4] },
      ],
    });

    expect(viewerChoice(open, undefined, ME)).toEqual([401]);
    expect(viewerChoice(open, { myOptionIds: [402], asOf: at(5) }, ME)).toEqual([401]);
    expect(viewerChoice(open, { myOptionIds: [402], asOf: at(10) }, ME)).toEqual([402]);

    const anonymous = { ...open, anonymous: true };

    expect(viewerChoice(anonymous, undefined, ME)).toBeNull();
    expect(viewerChoice(anonymous, { myOptionIds: [402], asOf: at(1) }, ME)).toEqual([402]);
  });

  it("closes a poll once closesAt passes, before the server says so", () => {
    const closing = poll(at(10), [0, 0], { closesAt: at(30) });

    expect(pollClosed(closing, Date.parse(at(29)))).toBe(false);
    expect(pollClosed(closing, Date.parse(at(30)))).toBe(true);
    expect(pollClosed(poll(at(10), [0, 0], { closed: true }), 0)).toBe(true);
  });

  it("moves the counts by the pending ballot without touching the poll", () => {
    const voted = poll(at(10), [1, 2], {
      options: [
        { id: 401, label: "Tea", votes: 1, voterIds: [ME] },
        { id: 402, label: "Coffee", votes: 2, voterIds: [3, 4] },
      ],
    });

    const view = pollView(voted, undefined, [402], ME, 0);

    expect(view.pending).toBe(true);
    expect(view.myOptionIds).toEqual([402]);
    expect(view.poll.options.map((option) => option.votes)).toEqual([0, 3]);
    expect(view.poll.options[1]?.voterIds).toEqual([3, 4, ME]);
    expect(view.poll.totalVotes).toBe(3);
    expect(voted.options[0]?.votes).toBe(1);

    const retracted = pollView(voted, undefined, [], ME, 0);

    expect(retracted.poll.totalVotes).toBe(2);
    expect(retracted.poll.options[0]?.voterIds).toEqual([]);
  });

  it("drops a pending ballot only when its own reply settles it", () => {
    const first = [401];
    const second = [402];
    const state = setPendingVote(setPendingVote(initialState, 40, first), 40, second);

    expect(settleVote(state, 40, first, null).cards.pendingVotes[40]).toBe(second);
    expect(settleVote(state, 40, second, null).cards.pendingVotes[40]).toBeUndefined();
  });

  it("keeps the newer ballot when an older one arrives", () => {
    const state = applyBallot(initialState, 40, [401], at(20));

    expect(applyBallot(state, 40, [402], at(19))).toBe(state);
  });
});

describe("previews", () => {
  it("keeps the shown value while loading again and fetches again when stale", () => {
    const now = Date.parse(at(0));
    const loaded = previewLoaded(initialState, "quotes", "5:3", 3, { state: "hidden" }, now);
    const reloading = previewLoading(loaded, "quotes", "5:3", 3);

    expect(reloading.cards.previews.quotes["5:3"]?.value).toEqual({ state: "hidden" });
    expect(needsFetch(reloading.cards.previews.quotes["5:3"], now + PREVIEW_TTL_MS * 2)).toBe(
      false,
    );

    expect(needsFetch(loaded.cards.previews.quotes["5:3"], now + 1000)).toBe(false);
    expect(needsFetch(loaded.cards.previews.quotes["5:3"], now + PREVIEW_TTL_MS + 1)).toBe(true);
    expect(needsFetch(undefined, now)).toBe(true);
  });

  it("moves the counts with the viewer's response", () => {
    const attendance = {
      eventId: 1,
      response: "maybe" as const,
      goingCount: 2,
      maybeCount: 1,
      declinedCount: 0,
      respondable: true,
      canApplyToFuture: false,
    };

    expect(withResponse(attendance, "going")).toMatchObject({
      response: "going",
      goingCount: 3,
      maybeCount: 0,
    });

    expect(withResponse(attendance, "maybe")).toBe(attendance);
  });
});

describe("visibleCards", () => {
  const link: MessageCard = {
    kind: "link",
    data: {
      url: "https://example.com",
      title: "Example",
      description: null,
      imageUrl: null,
      siteName: null,
    },
  };

  it("skips kinds this build doesn't know and link previews while embeds are suppressed", () => {
    // SAFETY: a card of a kind added after this build, as the wire may carry it.
    const future = JSON.parse('{"kind":"youtube","data":{"videoId":"x"}}') as MessageCard;
    const message = messageFixture(1, ROOM, { cards: [github(9), future, link] });

    expect(visibleCards(message)).toEqual([github(9), link]);
    expect(visibleCards({ ...message, embedsSuppressed: true })).toEqual([github(9)]);
  });
});
