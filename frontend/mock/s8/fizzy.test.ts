import { describe, expect, it } from "vitest";
import type { CreatedFizzyCard } from "../../src/gen/CreatedFizzyCard.ts";
import type { FizzyMessageCardForm } from "../../src/gen/FizzyMessageCardForm.ts";
import type { MessageDTO } from "../../src/gen/MessageDTO.ts";
import type { MessagePage } from "../../src/gen/MessagePage.ts";
import type { ThreadCreated } from "../../src/gen/ThreadCreated.ts";
import { field } from "../json.ts";
import { collect, errorOf, expectStatus, get, harness, messageBody, send } from "../s2/testing.ts";
import { SEED_IDS } from "../server.ts";

const VALIDATION = "Validation";

const ROOM = SEED_IDS.rooms.general;

const body = { boardId: "engineering", title: "  Track the fix  ", description: "Fix it" };

async function setup(threaded = false) {
  const { server } = harness();

  const message = await expectStatus<MessageDTO>(
    server,
    "POST",
    `/api/v1/rooms/${ROOM}/messages`,
    messageBody("fizzy-source", "The deploy is broken\nTrack the fix"),
    201,
  );

  let threadId: number | null = null;
  let source = message;

  if (threaded) {
    const created = await expectStatus<ThreadCreated>(
      server,
      "POST",
      `/api/v1/rooms/${ROOM}/threads`,
      {
        parentMessageId: message.id,
        name: "Deploy",
        message: messageBody("fizzy-thread-source", "Threaded problem"),
      },
      201,
    );

    threadId = created.detail.thread.id;
    source = created.message;
  }

  const path = `/api/v1/rooms/${ROOM}${threadId === null ? "" : `/threads/${threadId}`}/messages/${source.id}/fizzy_cards`;

  return { server, path, source, threadId };
}

describe("mock message-to-Fizzy", () => {
  it("serves the connected viewer's ordered flat boards, source and defaults", async () => {
    const { server, path, source } = await setup();
    const form = await get<FizzyMessageCardForm>(server, `${path}/new`);

    expect(form).toEqual({
      connected: true,
      boards: [
        { id: "engineering", name: "Engineering" },
        { id: "support", name: "Support" },
        { id: "roadmap", name: "Roadmap" },
      ],
      title: "The deploy is broken",
      description: `The deploy is broken\nTrack the fix\n\nSource: https://smartfire.test/rooms/${ROOM}/@${source.id}`,
      excerpt: "The deploy is broken\nTrack the fix",
      authorName: "Riel St. Amand",
      roomDisplayName: "general",
      fizzyUserName: "Riel St. Amand",
      accountName: "Smart Data",
    });
  });

  for (const threaded of [false, true]) {
    it(`posts the created card as a reply on the ${threaded ? "thread" : "room"} timeline and broadcasts it`, async () => {
      const { server, path, source, threadId } = await setup(threaded);
      const topic = threadId === null ? `room:${ROOM}` : `thread:${threadId}`;
      const events = collect(server, [topic]);
      const created = await expectStatus<CreatedFizzyCard>(server, "POST", path, body, 201);

      expect(created.number).toBe("580");
      expect(created.url).toBe("https://fizzy.test/897362094/cards/580");
      expect(created.notice).toBe("Fizzy card #580 created.");
      expect(created.message).toMatchObject({
        roomId: ROOM,
        threadId,
        replyToMessageId: source.id,
      });
      expect(created.message.markdownSource).toBe(
        `Created from https://smartfire.test/rooms/${ROOM}${threadId === null ? `/@${source.id}` : `?thread=${threadId}&message_id=${source.id}`}:\n${created.url}`,
      );

      const page = await get<MessagePage>(
        server,
        threadId === null
          ? `/api/v1/rooms/${ROOM}/messages`
          : `/api/v1/threads/${threadId}/messages`,
      );

      expect(page.messages.at(-1)).toEqual(created.message);
      expect(events).toContainEqual(
        expect.objectContaining({ topic, type: "message.created", data: created.message }),
      );
    });
  }

  it("returns structured blank-field errors and checks board failures before validation", async () => {
    const { server, path } = await setup();
    const response = await send(server, "POST", path, { ...body, boardId: "", title: " " });

    expect(response.status).toBe(422);
    expect(response.json).toEqual({
      error: {
        _tag: VALIDATION,
        message: "Choose a board and enter a title.",
        fields: { boardId: ["Choose a board."], title: ["Enter a title."] },
      },
    });
    await send(server, "POST", "/__mock/fizzy", { mode: "unreachable" });
    expect(errorOf((await send(server, "POST", path, { ...body, title: "" })).json).tag).toBe(
      "FizzyUnreachable",
    );
  });

  it("refuses a foreign thread, a message off the timeline, and a room the viewer cannot reach", async () => {
    const { server, path, threadId, source } = await setup(true);
    const wrongRoom = path.replace(`/rooms/${ROOM}`, `/rooms/${SEED_IDS.rooms.engineering}`);
    const rootPath = `/api/v1/rooms/${ROOM}/messages/${source.id}/fizzy_cards`;

    for (const endpoint of [
      wrongRoom,
      rootPath,
      `/api/v1/rooms/999999/threads/${threadId}/messages/${source.id}/fizzy_cards`,
    ]) {
      expect(
        errorOf((await server.handle({ method: "GET", path: `${endpoint}/new` })).json).tag,
      ).toBe("NotFound");
      expect(errorOf((await send(server, "POST", endpoint, body)).json).tag).toBe("NotFound");
    }
  });

  it("allows the form on a locked thread but refuses creation", async () => {
    const { server, path, threadId } = await setup(true);
    await send(server, "PATCH", `/api/v1/threads/${threadId}`, { status: "locked" });
    expect((await get<FizzyMessageCardForm>(server, `${path}/new`)).connected).toBe(true);
    const reply = await send(server, "POST", path, body);

    expect(reply.status).toBe(409);
    expect(errorOf(reply.json)).toEqual({
      tag: "FizzyThreadLocked",
      message: "This thread is locked",
    });
  });

  it("distinguishes failure modes and rejected-token disconnection", async () => {
    for (const [mode, tag] of [
      ["not-connected", "FizzyNotConnected"],
      ["unreachable", "FizzyUnreachable"],
      ["rejected", "FizzyTokenRejected"],
      ["read-only", "FizzyReadOnly"],
      ["probe-unreachable", "FizzyUnreachable"],
      ["refused", "FizzyRefused"],
      ["reply-failed", "FizzyReplyFailed"],
    ] satisfies [string, string][]) {
      const { server, path } = await setup();
      await send(server, "POST", "/__mock/fizzy", { mode });
      const response = await send(server, "POST", path, body);

      expect(errorOf(response.json).tag).toBe(tag);

      if (mode === "rejected" || mode === "not-connected") {
        const form = await get<FizzyMessageCardForm>(server, `${path}/new`);
        expect(form.connected).toBe(false);
        expect(form.boards).toEqual([]);
      }

      if (mode === "reply-failed")
        expect(field(field(response.json, "error"), "url")).toBe(
          "https://fizzy.test/897362094/cards/580",
        );
    }
  });

  it("marks a rejected board token disconnected and resets connection with the world", async () => {
    const { server, path } = await setup();
    await send(server, "POST", "/__mock/fizzy", { mode: "rejected" });
    expect(errorOf((await server.handle({ method: "GET", path: `${path}/new` })).json).tag).toBe(
      "FizzyTokenRejected",
    );
    expect((await get<FizzyMessageCardForm>(server, `${path}/new`)).connected).toBe(false);
    server.reset();
    const page = await get<MessagePage>(server, `/api/v1/rooms/${ROOM}/messages`);
    const source = page.messages.at(-1);

    expect(source).toBeDefined();
    expect(
      (
        await get<FizzyMessageCardForm>(
          server,
          `/api/v1/rooms/${ROOM}/messages/${source?.id}/fizzy_cards/new`,
        )
      ).connected,
    ).toBe(true);
  });
});
