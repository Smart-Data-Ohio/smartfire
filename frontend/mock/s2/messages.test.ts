import { describe, expect, it } from "vitest";
import type { ForwardDestinationList } from "../../src/gen/ForwardDestinationList.ts";
import type { ForwardResult } from "../../src/gen/ForwardResult.ts";
import type { MessageDTO } from "../../src/gen/MessageDTO.ts";
import type { MessageReactions } from "../../src/gen/MessageReactions.ts";
import type { MessageRead } from "../../src/gen/MessageRead.ts";
import type { MessageSource } from "../../src/gen/MessageSource.ts";
import type { PinList } from "../../src/gen/PinList.ts";
import type { PinState } from "../../src/gen/PinState.ts";
import type { RoomDetail } from "../../src/gen/RoomDetail.ts";
import type { SavedItem } from "../../src/gen/SavedItem.ts";
import { SEED_IDS } from "../server.ts";
import { MAX_PINS_PER_ROOM } from "./model.ts";
import { collect, errorOf, expectStatus, get, harness, messageBody, NOW, send } from "./testing.ts";

const { rooms, users, threads, messages, viewer } = SEED_IDS;

async function postOwn(server: Parameters<typeof send>[0], text: string): Promise<MessageDTO> {
  return expectStatus<MessageDTO>(
    server,
    "POST",
    `/api/v1/rooms/${rooms.quiet}/messages`,
    messageBody(`own-${text}`, text),
    201,
  );
}

describe("reading one message", () => {
  it("answers the message with its conversation, author and the viewer's save", async () => {
    const { server } = harness();
    const read = await get<MessageRead>(server, `/api/v1/messages/${messages.generalSaved}`);

    expect(read.message.id).toBe(messages.generalSaved);
    expect(read.conversation).toMatchObject({ roomId: rooms.general, threadId: null });
    expect(read.users.map((user) => user.id)).toContain(read.message.creatorId);
    expect(read.saved).toEqual({ messageId: messages.generalSaved, savedItemId: 1 });

    const reply = await get<MessageRead>(
      server,
      `/api/v1/messages/${messages.generalThreadViewerReply}`,
    );

    expect(reply.conversation.threadId).toBe(threads.generalActive);
    expect(reply.conversation.threadName).not.toBeNull();
  });

  it("is a 404 for a message out of reach", async () => {
    const { server } = harness();
    const response = await server.handle({ method: "GET", path: "/api/v1/messages/999999999" });

    expect(response.status).toBe(404);
    expect(errorOf(response.json).tag).toBe("NotFound");
  });
});

describe("editing and deleting", () => {
  it("edits the viewer's own message and publishes message.updated", async () => {
    const { server, clock } = harness();
    const own = await postOwn(server, "first draft");
    const events = collect(server, [`room:${rooms.quiet}`]);

    clock.advance(60_000);

    const edited = await expectStatus<MessageDTO>(
      server,
      "PATCH",
      `/api/v1/messages/${own.id}`,
      { markdownSource: "**final** draft" },
      200,
    );

    expect(edited.bodyHtml).toContain("<strong>final</strong>");
    expect(edited.editedAt).toBe(new Date(NOW + 60_000).toISOString());
    expect(events.map((event) => event.type)).toEqual(["message.updated"]);

    const source = await get<MessageSource>(server, `/api/v1/messages/${own.id}/source`);

    expect(source).toEqual({ messageId: own.id, markdownSource: "**final** draft" });

    const same = await expectStatus<MessageDTO>(
      server,
      "PATCH",
      `/api/v1/messages/${own.id}`,
      { markdownSource: "**final** draft" },
      200,
    );

    expect(same.editedAt).toBe(edited.editedAt);
  });

  it("refuses someone else's message, a blank body and an unknown id", async () => {
    const { server } = harness();
    const own = await postOwn(server, "mine");

    const other = await send(server, "PATCH", `/api/v1/messages/${messages.generalChart}`, {
      markdownSource: "hijacked",
    });

    const blank = await send(server, "PATCH", `/api/v1/messages/${own.id}`, {
      markdownSource: " ",
    });

    const missing = await send(server, "PATCH", "/api/v1/messages/999", { markdownSource: "x" });

    expect(other.status).toBe(403);
    expect(errorOf(other.json).tag).toBe("Forbidden");
    expect(blank.status).toBe(422);
    expect(missing.status).toBe(404);
  });

  it("deletes a message, unpinning it, and publishes message.removed", async () => {
    const { server } = harness();
    const events = collect(server, [`room:${rooms.general}`]);
    const response = await send(server, "DELETE", `/api/v1/messages/${messages.generalPinned}`);

    expect(response).toEqual({ status: 204, json: null });
    expect(events.map((event) => event.type)).toEqual([
      "message.removed",
      "message.pinned",
      "sidebar.row.upserted",
    ]);
    expect(events[0]?.data).toEqual({
      id: messages.generalPinned,
      roomId: rooms.general,
      threadId: null,
    });

    const detail = await get<RoomDetail>(server, `/api/v1/rooms/${rooms.general}`);

    expect(detail.pinsCount).toBe(2);
    expect(
      (await send(server, "DELETE", `/api/v1/messages/${messages.generalPinned}`)).status,
    ).toBe(404);
  });

  it("deletes a thread reply and refreshes the parent's indicator", async () => {
    const { server } = harness();
    const events = collect(server, [`room:${rooms.general}`, `thread:${threads.generalActive}`]);

    await send(server, "DELETE", `/api/v1/messages/${messages.generalThreadFunnel}`);

    expect(events.map((event) => event.type)).toEqual([
      "message.removed",
      "thread.indicator",
      "thread.updated",
      "thread.updated",
      "thread.unread",
    ]);
    expect(events[1]?.data).toMatchObject({ thread: { replyCount: 5 } });
  });
});

describe("reactions and boosts", () => {
  it("toggles a reaction and fills in its title", async () => {
    const { server } = harness();
    const events = collect(server, [`room:${rooms.general}`]);
    const path = `/api/v1/messages/${messages.generalChart}/boosts`;

    const added = await expectStatus<MessageReactions>(
      server,
      "POST",
      path,
      { content: "👍" },
      200,
    );

    expect(added.reactions.at(-1)).toEqual({
      content: "👍",
      title: "Thumbs up",
      imageUrl: null,
      reactorIds: [viewer],
    });
    expect(events.map((event) => event.type)).toEqual(["message.reactions"]);

    const shortcode = await expectStatus<MessageReactions>(
      server,
      "POST",
      path,
      { content: ":thumbsup:" },
      200,
    );

    expect(shortcode.reactions.some((pill) => pill.content === "👍")).toBe(false);

    const named = await expectStatus<MessageReactions>(
      server,
      "POST",
      path,
      { content: "🦀" },
      200,
    );

    expect(named.reactions.at(-1)?.title).toBe("crab");
  });

  it("adds a free-text boost, removes it, and refuses bad content", async () => {
    const { server } = harness();
    const path = `/api/v1/messages/${messages.generalChart}/boosts`;

    const boosted = await expectStatus<MessageReactions>(
      server,
      "POST",
      path,
      { content: " nice! " },
      200,
    );

    const boost = boosted.boosts.at(-1);

    expect(boost).toMatchObject({ boosterId: viewer, content: "nice!" });

    const blank = await send(server, "POST", path, { content: "  " });
    const long = await send(server, "POST", path, { content: "x".repeat(17) });

    expect(blank.status).toBe(422);
    expect(long.status).toBe(422);
    expect(errorOf(long.json).tag).toBe("Validation");

    const removed = await expectStatus<MessageReactions>(
      server,
      "DELETE",
      `${path}/${boost?.id ?? 0}`,
      null,
      200,
    );

    expect(removed.boosts).toHaveLength(0);

    const othersBoost = await expectStatus<MessageReactions>(
      server,
      "GET",
      "/api/v1/messages/0",
      null,
      404,
    );

    expect(othersBoost).toBeDefined();
  });

  it("refuses removing someone else's boost", async () => {
    const { server } = harness();
    const path = `/api/v1/messages/${messages.generalBoosts}/boosts/1`;

    expect((await send(server, "DELETE", path)).status).toBe(403);
  });

  it("lets a control react for someone else", async () => {
    const { server } = harness();

    const response = await send(server, "POST", "/__mock/react", {
      messageId: messages.generalChart,
      userId: users.grace,
      content: ":smartfire:",
    });

    expect(response.status).toBe(200);
    expect(response.json).toMatchObject({
      reactions: [
        {},
        {},
        { content: ":smartfire:", imageUrl: "/icons/smartfire", reactorIds: [8] },
      ],
    });
  });
});

describe("pins", () => {
  it("pins and unpins, publishing message.pinned", async () => {
    const { server } = harness();
    const events = collect(server, [`room:${rooms.general}`]);
    const path = `/api/v1/messages/${messages.generalChart}/pin`;
    const pinned = await expectStatus<PinState>(server, "POST", path, null, 201);

    expect(pinned).toEqual({
      messageId: messages.generalChart,
      roomId: rooms.general,
      pinned: true,
      pinCount: 4,
    });
    expect(events.map((event) => event.type)).toEqual(["message.pinned"]);

    await expectStatus<PinState>(server, "POST", path, null, 201);
    expect(events).toHaveLength(1);

    const list = await get<PinList>(server, `/api/v1/rooms/${rooms.general}/pins`);

    expect(list.pins[0]?.messageId).toBe(messages.generalChart);

    const unpinned = await expectStatus<PinState>(server, "DELETE", path, null, 200);

    expect(unpinned).toMatchObject({ pinned: false, pinCount: 3 });
  });

  it("refuses the 51st pin in a room", async () => {
    const { server } = harness();
    // The oldest #general messages: none of them is pinned in the seed, which pins 3.
    const ids = Array.from({ length: MAX_PINS_PER_ROOM - 2 }, (_, index) => 10_000 + index);

    for (const id of ids.slice(0, -1)) {
      await expectStatus(server, "POST", `/api/v1/messages/${id}/pin`, null, 201);
    }

    const response = await send(server, "POST", `/api/v1/messages/${ids.at(-1) ?? 0}/pin`);

    expect(response.status).toBe(422);
  });
});

describe("saved items", () => {
  it("saves with a reminder, updates it, and unsaves", async () => {
    const { server } = harness();
    const events = collect(server);
    const remindAt = new Date(NOW + 3_600_000).toISOString();

    const item = await expectStatus<SavedItem>(
      server,
      "POST",
      "/api/v1/saved",
      { messageId: messages.generalChart, remindAt },
      201,
    );

    expect(item).toMatchObject({
      messageId: messages.generalChart,
      status: "in_progress",
      remindAt,
    });
    expect(events.at(-1)).toMatchObject({
      type: "saved.changed",
      data: { messageId: messages.generalChart, item },
    });

    const again = await expectStatus<SavedItem>(
      server,
      "POST",
      "/api/v1/saved",
      { messageId: messages.generalChart, remindAt: null },
      201,
    );

    expect(again).toMatchObject({ id: item.id, remindAt: null });
    expect((await send(server, "DELETE", `/api/v1/saved/${item.id}`)).status).toBe(204);
    expect(events.at(-1)?.data).toEqual({ messageId: messages.generalChart, item: null });
    expect((await send(server, "DELETE", `/api/v1/saved/${item.id}`)).status).toBe(404);
  });

  it("refuses a reminder in the past", async () => {
    const { server } = harness();

    const response = await send(server, "POST", "/api/v1/saved", {
      messageId: messages.generalChart,
      remindAt: new Date(NOW - 1000).toISOString(),
    });

    expect(response.status).toBe(422);
  });
});

describe("forwards", () => {
  it("lists rooms by name with their open threads", async () => {
    const { server } = harness();
    const list = await get<ForwardDestinationList>(server, "/api/v1/forward_destinations");
    const general = list.destinations.find((destination) => destination.roomId === rooms.general);
    const maya = list.destinations.find((destination) => destination.roomId === rooms.dmMaya);

    expect(general?.threads.map((thread) => thread.id)).toEqual([
      threads.generalActive,
      threads.generalClosed,
    ]);
    expect(maya).toMatchObject({ direct: true, threads: [], name: "Maya Okafor" });
  });

  it("forwards to a room and a thread with a note", async () => {
    const { server } = harness();
    const events = collect(server, [`room:${rooms.design}`, `thread:${threads.generalActive}`]);

    const result = await expectStatus<ForwardResult>(
      server,
      "POST",
      `/api/v1/messages/${messages.generalChart}/forwards`,
      {
        note: "For the review",
        destinations: [
          { roomId: rooms.design, threadId: null },
          { roomId: rooms.general, threadId: threads.generalActive },
        ],
      },
      201,
    );

    expect(result.forwards).toHaveLength(2);
    expect(result.forwards[0]).toMatchObject({
      roomId: rooms.design,
      threadId: null,
      forwardedFromMessageId: messages.generalChart,
      forwardNote: "For the review",
      attachment: { filename: "signups-by-week.svg" },
    });
    expect(result.forwards[1]?.threadId).toBe(threads.generalActive);
    expect(events.filter((event) => event.type === "message.created")).toHaveLength(2);
  });

  it("refuses bad destinations", async () => {
    const { server } = harness();
    const path = `/api/v1/messages/${messages.generalChart}/forwards`;
    const none = await send(server, "POST", path, { note: null, destinations: [] });

    const locked = await send(server, "POST", path, {
      note: null,
      destinations: [{ roomId: rooms.general, threadId: threads.generalLocked }],
    });

    const repeated = await send(server, "POST", path, {
      note: null,
      destinations: [
        { roomId: rooms.design, threadId: null },
        { roomId: rooms.design, threadId: null },
      ],
    });

    const tooMany = await send(server, "POST", path, {
      note: null,
      destinations: Array.from({ length: 6 }, (_, index) => ({
        roomId: index + 1,
        threadId: null,
      })),
    });

    expect([none.status, locked.status, repeated.status, tooMany.status]).toEqual([
      422, 422, 422, 422,
    ]);
  });
});
