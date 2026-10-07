import { describe, expect, it } from "vitest";
import type { EventAttendance } from "../../src/gen/EventAttendance.ts";
import type { FizzyCardPreview } from "../../src/gen/FizzyCardPreview.ts";
import type { GithubPullRequestCard } from "../../src/gen/GithubPullRequestCard.ts";
import type { MessageDTO } from "../../src/gen/MessageDTO.ts";
import type { MessagePage } from "../../src/gen/MessagePage.ts";
import type { PollResults } from "../../src/gen/PollResults.ts";
import type { QuotePreviewResult } from "../../src/gen/QuotePreviewResult.ts";
import type { Sidebar } from "../../src/gen/Sidebar.ts";
import type { ThreadCreated } from "../../src/gen/ThreadCreated.ts";
import {
  collect,
  errorOf,
  expectStatus,
  get,
  harness,
  messageBody,
  NOW,
  send,
} from "../s2/testing.ts";
import { SEED_IDS } from "../server.ts";

const { cards, users, viewer } = SEED_IDS;

const ROOM = cards.room;

const HOUR = 3_600_000;

async function page(server: ReturnType<typeof harness>["server"]): Promise<MessagePage> {
  return get<MessagePage>(server, `/api/v1/rooms/${ROOM}/messages`);
}

function held(list: MessagePage, id: number): MessageDTO {
  const found = list.messages.find((message) => message.id === id);

  if (found === undefined) throw new Error(`message ${id} is missing`);

  return found;
}

describe("the cards seed", () => {
  it("adds #product-updates, read, with a message for every card kind", async () => {
    const { server } = harness();
    const sidebar = await get<Sidebar>(server, "/api/v1/sidebar");
    const row = sidebar.rows.find((candidate) => candidate.room.id === ROOM);

    expect(row).toMatchObject({ displayName: "product-updates", unreadCount: 0, mentionCount: 0 });

    const list = await page(server);

    const kinds = new Set(
      list.messages.flatMap((message) => message.cards.map((card) => card.kind)),
    );

    expect([...kinds].sort()).toEqual(
      ["drive", "event", "fizzy", "github", "link", "linkedin", "quote", "x", "youtube"].sort(),
    );

    expect(list.messages.at(-1)?.id).toBe(cards.messages.pollOpen);
    expect(held(list, cards.messages.suppressed)).toMatchObject({ embedsSuppressed: true });
    expect(held(list, cards.messages.suppressed).cards.map((card) => card.kind)).toEqual([
      "github",
    ]);
  });

  it("seeds open, closed, multiple and anonymous polls", async () => {
    const { server } = harness();
    const list = await page(server);
    const poll = (id: number) => held(list, id).poll;

    expect(poll(cards.messages.pollClosed)).toMatchObject({ closed: true, totalVotes: 5 });
    expect(poll(cards.messages.pollMultiple)).toMatchObject({ multiple: true, closed: false });
    expect(poll(cards.messages.pollAnonymous)?.anonymous).toBe(true);

    expect(poll(cards.messages.pollAnonymous)?.options.every((o) => o.voterIds.length === 0)).toBe(
      true,
    );

    expect(poll(cards.messages.pollOpen)?.options.flatMap((o) => o.voterIds)).not.toContain(viewer);
  });
});

describe("polls", () => {
  it("answers the viewer's own choice, anonymous polls included", async () => {
    const { server } = harness();

    const results = await get<PollResults>(
      server,
      `/api/v1/rooms/${ROOM}/polls/${cards.polls.anonymous}`,
    );

    expect(results.myOptionIds).toEqual([cards.polls.anonymous * 10 + 1]);
    expect(results.poll.options.flatMap((option) => option.voterIds)).toEqual([]);
  });

  it("replaces the ballot, publishing poll.updated and poll.ballot", async () => {
    const { server } = harness();
    const events = collect(server, [`room:${ROOM}`]);
    const tacos = cards.polls.open * 10 + 1;

    const results = await expectStatus<PollResults>(
      server,
      "POST",
      `/api/v1/rooms/${ROOM}/polls/${cards.polls.open}/vote`,
      { optionIds: [tacos] },
      200,
    );

    expect(results.myOptionIds).toEqual([tacos]);
    expect(results.poll.totalVotes).toBe(5);
    expect(results.poll.options[0]?.voterIds).toContain(viewer);
    expect(events.map((event) => event.type)).toEqual(["poll.updated", "poll.ballot"]);
    expect(events[1]).toMatchObject({ topic: "user", data: { myOptionIds: [tacos] } });

    const retracted = await expectStatus<PollResults>(
      server,
      "POST",
      `/api/v1/rooms/${ROOM}/polls/${cards.polls.open}/vote`,
      { optionIds: [] },
      200,
    );

    expect(retracted.poll.totalVotes).toBe(4);
    expect(retracted.poll.asOf > results.poll.asOf).toBe(true);
    expect(held(await page(server), cards.messages.pollOpen).poll?.totalVotes).toBe(4);
  });

  it("refuses votes on a closed poll, foreign options and two picks in a single-choice poll", async () => {
    const { server } = harness();

    const vote = (pollId: number, optionIds: number[]) =>
      send(server, "POST", `/api/v1/rooms/${ROOM}/polls/${pollId}/vote`, { optionIds });

    const closed = await vote(cards.polls.closed, [cards.polls.closed * 10 + 1]);
    const foreign = await vote(cards.polls.open, [9999]);

    const two = await vote(cards.polls.open, [
      cards.polls.open * 10 + 1,
      cards.polls.open * 10 + 2,
    ]);

    expect([closed.status, foreign.status, two.status]).toEqual([422, 422, 422]);
    expect(errorOf(closed.json).message).toContain("This poll is closed");
  });

  it("closes a poll on its own once closesAt passes", async () => {
    const { server, clock } = harness();

    clock.advance(3 * HOUR);

    const results = await get<PollResults>(
      server,
      `/api/v1/rooms/${ROOM}/polls/${cards.polls.anonymous}`,
    );

    expect(results.poll.closed).toBe(true);
  });

  it("creates a poll idempotently and validates it", async () => {
    const { server } = harness();
    const events = collect(server, [`room:${ROOM}`]);

    const body = {
      clientMessageId: "poll-1",
      question: "Ship on Friday?",
      options: ["Yes", " ", "No"],
      multiple: false,
      anonymous: true,
      closesAt: new Date(NOW + HOUR).toISOString(),
    };

    const created = await expectStatus<MessageDTO>(
      server,
      "POST",
      `/api/v1/rooms/${ROOM}/polls`,
      body,
      201,
    );

    expect(created.poll?.options.map((option) => option.label)).toEqual(["Yes", "No"]);
    expect(created.creatorId).toBe(viewer);
    expect(events[0]).toMatchObject({ type: "message.created", data: { id: created.id } });
    expect(events[0]?.type === "message.created" && events[0].data.poll?.id).toBe(created.poll?.id);

    const again = await expectStatus<MessageDTO>(
      server,
      "POST",
      `/api/v1/rooms/${ROOM}/polls`,
      body,
      200,
    );

    expect(again.id).toBe(created.id);

    const invalid = [
      { ...body, clientMessageId: "poll-2", question: " " },
      { ...body, clientMessageId: "poll-3", options: ["Only one"] },
      { ...body, clientMessageId: "poll-4", closesAt: new Date(NOW - HOUR).toISOString() },
    ];

    for (const candidate of invalid) {
      expect((await send(server, "POST", `/api/v1/rooms/${ROOM}/polls`, candidate)).status).toBe(
        422,
      );
    }
  });

  it("lets another person vote through the control endpoint", async () => {
    const { server } = harness();
    const events = collect(server, [`room:${ROOM}`]);

    await expectStatus(
      server,
      "POST",
      "/__mock/cards",
      {
        op: "vote",
        pollId: cards.polls.open,
        userId: users.sam,
        optionIds: [cards.polls.open * 10 + 4],
      },
      200,
    );

    expect(events.map((event) => event.type)).toEqual(["poll.updated"]);
  });
});

describe("events", () => {
  it("reads and records the viewer's response", async () => {
    const { server } = harness();
    const path = `/api/v1/rooms/${ROOM}/events/${cards.events.recurring}/attendance`;
    const before = await get<EventAttendance>(server, path);

    expect(before).toMatchObject({
      response: null,
      goingCount: 2,
      respondable: true,
      canApplyToFuture: true,
    });

    const after = await expectStatus<EventAttendance>(
      server,
      "PUT",
      path,
      { response: "going", applyToFuture: true },
      200,
    );

    expect(after).toMatchObject({ response: "going", goingCount: 3 });
  });

  it("refuses a response to a cancelled event", async () => {
    const { server } = harness();
    const path = `/api/v1/rooms/${ROOM}/events/${cards.events.cancelled}/attendance`;

    expect((await get<EventAttendance>(server, path)).respondable).toBe(false);
    expect(
      (await send(server, "PUT", path, { response: "maybe", applyToFuture: false })).status,
    ).toBe(403);
  });
});

describe("previews", () => {
  const github = (server: ReturnType<typeof harness>["server"], id: number, query: string) =>
    get<GithubPullRequestCard>(
      server,
      `/api/v1/rooms/${ROOM}/github/pull_requests/${id}/card?${query}`,
    );

  it("serves every GitHub state, and 404 for a message that doesn't link the pull request", async () => {
    const { server } = harness();
    const { pullRequests: pr, messages } = cards;

    expect(await github(server, pr.open, `messageId=${messages.githubOpen}`)).toMatchObject({
      state: "loaded",
      review: "approved",
      checks: "passing",
      discussionThreadId: null,
      files: null,
    });

    expect(
      (await github(server, pr.loading, `messageId=${messages.githubDraftAndLoading}`)).state,
    ).toBe("loading");
    expect(
      (await github(server, pr.failed, `messageId=${messages.githubFailedAndHidden}`)).state,
    ).toBe("failed");
    expect(
      (await github(server, pr.hidden, `messageId=${messages.githubFailedAndHidden}`)).state,
    ).toBe("hidden");

    const wrong = await server.handle({
      method: "GET",
      path: `/api/v1/rooms/${ROOM}/github/pull_requests/${pr.open}/card?messageId=${messages.drive}`,
    });

    expect(wrong.status).toBe(404);
  });

  it("lists files for the thread started on the pull request's message", async () => {
    const { server } = harness();
    const { pullRequests: pr, messages } = cards;

    const created = await expectStatus<ThreadCreated>(
      server,
      "POST",
      `/api/v1/rooms/${ROOM}/threads`,
      { parentMessageId: messages.githubOpen, name: null, message: messageBody("t-1", "Discuss") },
      201,
    );

    const threadId = created.detail.thread.id;
    const header = await github(server, pr.open, `threadId=${threadId}`);

    expect(header).toMatchObject({ state: "loaded", discussionThreadId: threadId });
    expect(header.state === "loaded" && header.files?.totalCount).toBe(4);

    expect(await github(server, pr.open, `messageId=${messages.githubOpen}`)).toMatchObject({
      discussionThreadId: threadId,
      files: null,
    });

    const other = await server.handle({
      method: "GET",
      path: `/api/v1/rooms/${ROOM}/github/pull_requests/${pr.merged}/card?threadId=${threadId}`,
    });

    expect(other.status).toBe(404);
  });

  it("serves every Fizzy state", async () => {
    const { server } = harness();
    const { fizzy, messages } = cards;

    const state = async (id: number, messageId: number) =>
      (
        await get<FizzyCardPreview>(
          server,
          `/api/v1/rooms/${ROOM}/fizzy/cards/${id}/card?messageId=${messageId}`,
        )
      ).state;

    expect(await state(fizzy.loaded, messages.fizzyLoaded)).toBe("loaded");
    expect(await state(fizzy.notConnected, messages.fizzyNotConnectedAndNotFound)).toBe(
      "not_connected",
    );
    expect(await state(fizzy.notFound, messages.fizzyNotConnectedAndNotFound)).toBe("not_found");
    expect(await state(fizzy.failed, messages.fizzyFailedAndLoading)).toBe("failed");
    expect(await state(fizzy.loading, messages.fizzyFailedAndLoading)).toBe("loading");
  });

  it("serves quotes the viewer may see, and hides the rest", async () => {
    const { server } = harness();

    const quote = (id: number) =>
      get<QuotePreviewResult>(server, `/api/v1/rooms/${ROOM}/message_links/${id}/card`);

    expect(await quote(cards.references.fetched)).toMatchObject({
      state: "loaded",
      roomLabel: "#general",
    });

    expect(await quote(cards.references.hidden)).toEqual({ state: "hidden" });
  });

  it("republishes a message's cards with a later asOf", async () => {
    const { server, clock } = harness();
    const events = collect(server, [`room:${ROOM}`]);

    clock.advance(1000);
    await expectStatus(
      server,
      "POST",
      "/__mock/cards",
      { op: "refresh", messageId: cards.messages.githubOpen },
      200,
    );

    expect(events[0]).toMatchObject({
      type: "message.cards",
      data: { messageId: cards.messages.githubOpen, asOf: new Date(NOW + 1000).toISOString() },
    });

    expect(held(await page(server), cards.messages.githubOpen).cardsAsOf).toBe(
      new Date(NOW + 1000).toISOString(),
    );
  });
});
