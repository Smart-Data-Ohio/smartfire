import { describe, expect, it } from "@effect/vitest";
import { Schema } from "effect";
import { collect, harness, send } from "../../../mock/s2/testing.ts";
import { CARD_IDS } from "../../../mock/s3/cards.ts";
import type { MockServer } from "../../../mock/server.ts";
import {
  EventAttendance,
  FizzyCardPreview,
  GithubPullRequestCard,
  PollResults,
  QuotePreviewResult,
} from "./cards.ts";
import { MessageDTO, MessagePage } from "./message.ts";
import { SyncEvent } from "./sync.ts";

const ROOM = CARD_IDS.room;

async function json(server: MockServer, path: string) {
  const response = await server.handle({ method: "GET", path: `/api/v1${path}` });

  expect(response.status, path).toBe(200);

  return response.json;
}

// The mock serves the cards the SPA reads; each reply must pass the pinned schema as it would
// in the browser, so the mock can't drift from the contract.
describe("the cards mock against the pinned schemas", () => {
  it("serves a page whose every card and poll decodes, the future kind as unknown", async () => {
    const { server } = harness();

    const page = Schema.decodeUnknownSync(MessagePage)(
      await json(server, `/rooms/${ROOM}/messages`),
    );

    const unknown = page.messages.find((message) => message.id === CARD_IDS.messages.unknownKind);

    expect(unknown?.cards).toEqual([{ kind: "unknown", originalKind: "youtube" }]);
    expect(
      page.messages.flatMap((message) => message.cards).filter((card) => card.kind === "unknown"),
    ).toHaveLength(1);
  });

  it("serves previews, results and attendance that decode", async () => {
    const { server } = harness();
    const { messages, pullRequests, fizzy, references, polls, events } = CARD_IDS;

    const github = [
      [pullRequests.open, messages.githubOpen],
      [pullRequests.merged, messages.githubMerged],
      [pullRequests.draft, messages.githubDraftAndLoading],
      [pullRequests.loading, messages.githubDraftAndLoading],
      [pullRequests.failed, messages.githubFailedAndHidden],
      [pullRequests.hidden, messages.githubFailedAndHidden],
    ] as const;

    for (const [id, messageId] of github) {
      const body = await json(
        server,
        `/rooms/${ROOM}/github/pull_requests/${id}/card?messageId=${messageId}`,
      );

      Schema.decodeUnknownSync(GithubPullRequestCard)(body);
    }

    const fizzyCards = [
      [fizzy.loaded, messages.fizzyLoaded],
      [fizzy.notConnected, messages.fizzyNotConnectedAndNotFound],
      [fizzy.notFound, messages.fizzyNotConnectedAndNotFound],
      [fizzy.failed, messages.fizzyFailedAndLoading],
      [fizzy.loading, messages.fizzyFailedAndLoading],
    ] as const;

    for (const [id, messageId] of fizzyCards) {
      Schema.decodeUnknownSync(FizzyCardPreview)(
        await json(server, `/rooms/${ROOM}/fizzy/cards/${id}/card?messageId=${messageId}`),
      );
    }

    for (const id of [references.fetched, references.hidden]) {
      Schema.decodeUnknownSync(QuotePreviewResult)(
        await json(server, `/rooms/${ROOM}/message_links/${id}/card`),
      );
    }

    for (const id of Object.values(polls)) {
      Schema.decodeUnknownSync(PollResults)(await json(server, `/rooms/${ROOM}/polls/${id}`));
    }

    for (const id of Object.values(events)) {
      Schema.decodeUnknownSync(EventAttendance)(
        await json(server, `/rooms/${ROOM}/events/${id}/attendance`),
      );
    }
  });

  it("publishes poll and card events that decode", async () => {
    const { server } = harness();
    const published = collect(server, [`room:${ROOM}`]);

    await send(server, "POST", `/api/v1/rooms/${ROOM}/polls/${CARD_IDS.polls.open}/vote`, {
      optionIds: [CARD_IDS.polls.open * 10 + 2],
    });

    await send(server, "POST", "/__mock/cards", {
      op: "refresh",
      messageId: CARD_IDS.messages.githubOpen,
    });

    const created = await send(server, "POST", `/api/v1/rooms/${ROOM}/polls`, {
      clientMessageId: "decode-1",
      question: "Decode?",
      options: ["Yes", "No"],
      multiple: true,
      anonymous: false,
      closesAt: null,
    });

    Schema.decodeUnknownSync(MessageDTO)(created.json);

    expect(published.map((event) => event.type)).toEqual([
      "poll.updated",
      "poll.ballot",
      "message.cards",
      "message.created",
      "sidebar.row.upserted",
    ]);

    for (const event of published) {
      Schema.decodeUnknownSync(SyncEvent)(event);
    }
  });
});
