import { afterEach, describe, expect, it, vi } from "vitest";
import type { BoardListing } from "../../src/gen/BoardListing.ts";
import type { BoardPostForm } from "../../src/gen/BoardPostForm.ts";
import type { BoardTagCatalog } from "../../src/gen/BoardTagCatalog.ts";
import type { ThreadDetail } from "../../src/gen/ThreadDetail.ts";
import { HttpError, type MockResponse, notFound } from "../http.ts";
import { field, type Json } from "../json.ts";
import { dispatch } from "../s2/context.ts";
import { collect, errorOf, expectStatus, get, harness, NOW, send } from "../s2/testing.ts";
import { buildWorld } from "../s3/seed.ts";
import { USER_IDS, VIEWER_ID } from "../seed.ts";
import { createBoardTags } from "./board-tags.ts";
import { BOARD_ROOM_ID } from "./seed.ts";

afterEach(() => {
  vi.restoreAllMocks();
});

const root = `/api/v1/rooms/${BOARD_ROOM_ID}`;

const post = (tags: Json[]) => ({
  name: "Tagged post",
  status: "planned",
  ownerId: null,
  tags,
  message: null,
});

function fieldsOf(response: MockResponse): Json | undefined {
  return field(field(response.json, "error"), "fields");
}

async function seedCatalog(server: ReturnType<typeof harness>["server"], body: Json) {
  return expectStatus<BoardTagCatalog>(server, "POST", "/__mock/board-tags", body, 200);
}

describe("mock board tag catalog", () => {
  it("creates, renames, reorders and deletes tags and tells the room each time", async () => {
    const { server } = harness();
    const events = collect(server, [`room:${BOARD_ROOM_ID}`]);

    const created = await expectStatus<BoardTagCatalog>(
      server,
      "POST",
      `${root}/board/tags`,
      { name: "  Bug report ", emoji: "🐞" },
      201,
    );

    expect(created.tags).toEqual([
      { id: expect.any(Number), name: "Bug report", emoji: "🐞", position: 0 },
    ]);

    const second = await expectStatus<BoardTagCatalog>(
      server,
      "POST",
      `${root}/board/tags`,
      { name: "Idea", emoji: "" },
      201,
    );

    const [bug, idea] = second.tags;

    if (bug === undefined || idea === undefined) throw new Error("Tags missing");

    expect(idea.emoji).toBeNull();

    const reordered = await expectStatus<BoardTagCatalog>(
      server,
      "PUT",
      `${root}/board/tags/order`,
      { tagIds: [idea.id, bug.id] },
      200,
    );

    expect(reordered.tags.map((tag) => tag.name)).toEqual(["Idea", "Bug report"]);

    const renamed = await expectStatus<BoardTagCatalog>(
      server,
      "PATCH",
      `${root}/board/tags/${bug.id}`,
      { name: "Bug", emoji: "🐛" },
      200,
    );

    expect(renamed.tags[1]).toMatchObject({ name: "Bug", emoji: "🐛", position: 1 });

    const deleted = await expectStatus<BoardTagCatalog>(
      server,
      "DELETE",
      `${root}/board/tags/${idea.id}`,
      null,
      200,
    );

    expect(deleted.tags.map((tag) => tag.name)).toEqual(["Bug"]);
    expect(await get<BoardTagCatalog>(server, `${root}/board/tags`)).toEqual(deleted);
    expect(events.filter((event) => event.type === "board.automations.changed")).toHaveLength(5);
  });

  it("refuses blank, long, duplicate and surplus names as the server says them", async () => {
    const { server } = harness();

    await seedCatalog(server, { tags: [{ name: "Bug" }] });

    const blank = await send(server, "POST", `${root}/board/tags`, { name: "  " });

    expect(blank.status).toBe(422);
    expect(fieldsOf(blank)).toEqual({ name: ["can't be blank"] });

    const long = await send(server, "POST", `${root}/board/tags`, { name: "x".repeat(21) });

    expect(fieldsOf(long)).toEqual({ name: ["is too long (maximum is 20 characters)"] });

    const taken = await send(server, "POST", `${root}/board/tags`, { name: "BUG" });

    expect(fieldsOf(taken)).toEqual({ name: ["has already been taken"] });
    expect(errorOf(taken.json).message).toBe("Validation failed: Name has already been taken");

    await seedCatalog(server, {
      tags: Array.from({ length: 20 }, (_, index) => ({ name: `Tag ${index}` })),
    });

    const surplus = await send(server, "POST", `${root}/board/tags`, { name: "One more" });

    expect(fieldsOf(surplus)).toEqual({ tags: ["are limited to 20 per board"] });

    const partial = await send(server, "PUT", `${root}/board/tags/order`, { tagIds: [1, 2] });

    expect(fieldsOf(partial)).toEqual({ tag_ids: ["must contain every board tag exactly once"] });

    const stranger = await send(server, "PATCH", `${root}/board`, {
      tagsRequired: true,
      defaultBoardTagId: 1,
    });

    expect(fieldsOf(stranger)).toEqual({ default_board_tag_id: ["must belong to this board"] });
  });

  it("carries the catalog on the listing and the new-post form", async () => {
    const { server } = harness();

    const seeded = await seedCatalog(server, {
      tags: [{ name: "Bug", emoji: "🐞" }, { name: "Idea" }],
      tagsRequired: true,
      defaultTag: "idea",
    });

    const idea = seeded.tags[1];
    const listing = await get<BoardListing>(server, `${root}/board`);
    const form = await get<BoardPostForm>(server, `${root}/posts/new`);

    expect(idea?.name).toBe("Idea");

    for (const answer of [listing, form]) {
      expect(answer.tags).toEqual(seeded.tags);
      expect(answer.tagsRequired).toBe(true);
      expect(answer.defaultBoardTagId).toBe(idea?.id);
    }
  });

  it("resolves catalog names, falls back to the default and refuses without one", async () => {
    const { server } = harness();

    await seedCatalog(server, {
      tags: [{ name: "Bug report" }, { name: "Idea" }],
      tagsRequired: true,
      defaultTag: "Idea",
    });

    const named = await expectStatus<ThreadDetail>(
      server,
      "POST",
      `${root}/posts`,
      post(["bug REPORT", "api"]),
      201,
    );

    expect(named.thread.work?.tags).toEqual(["Bug report", "api"]);

    const fallback = await expectStatus<ThreadDetail>(
      server,
      "POST",
      `${root}/posts`,
      post(["api"]),
      201,
    );

    expect(fallback.thread.work?.tags).toEqual(["Idea", "api"]);

    const filtered = await get<BoardListing>(server, `${root}/board?tag=bug%20report`);

    expect(filtered.tag).toBe("Bug report");
    expect(filtered.posts.map(({ thread }) => thread.id)).toEqual([named.thread.id]);

    await seedCatalog(server, { tags: [{ name: "Bug report" }], tagsRequired: true });

    const refused = await send(server, "POST", `${root}/posts`, post(["api"]));

    expect(refused.status).toBe(422);
    expect(fieldsOf(refused)).toEqual({ tags: ["must include a catalog tag"] });

    const edit = await send(server, "PATCH", `/api/v1/threads/${named.thread.id}/work`, {
      tags: ["api"],
    });

    expect(fieldsOf(edit)).toEqual({ tags: ["must include a catalog tag"] });
  });

  it("follows a rename and a delete through the posts and drops a deleted default", async () => {
    const { server } = harness();

    const seeded = await seedCatalog(server, {
      tags: [{ name: "Bug" }, { name: "Idea" }],
      tagsRequired: true,
      defaultTag: "Bug",
    });

    const [bug, idea] = seeded.tags;

    if (bug === undefined || idea === undefined) throw new Error("Tags missing");

    const made = await expectStatus<ThreadDetail>(
      server,
      "POST",
      `${root}/posts`,
      post(["bug", "idea"]),
      201,
    );

    await send(server, "PATCH", `${root}/board/tags/${idea.id}`, { name: "Proposal" });

    const deleted = await expectStatus<BoardTagCatalog>(
      server,
      "DELETE",
      `${root}/board/tags/${bug.id}`,
      null,
      200,
    );

    expect(deleted.defaultBoardTagId).toBeNull();
    expect(deleted.tagsRequired).toBe(true);

    const listing = await get<BoardListing>(server, `${root}/board?status=all`);

    expect(
      listing.posts.find(({ thread }) => thread.id === made.thread.id)?.thread.work?.tags,
    ).toEqual(["Proposal"]);
  });

  it("lifts the requirement with the last tag and refuses one with nothing to meet it", async () => {
    const { server } = harness();

    const seeded = await seedCatalog(server, {
      tags: [{ name: "Bug" }, { name: "Idea" }],
      tagsRequired: true,
      defaultTag: "Idea",
    });

    const [bug, idea] = seeded.tags;

    if (bug === undefined || idea === undefined) throw new Error("Tags missing");

    const one = await expectStatus<BoardTagCatalog>(
      server,
      "DELETE",
      `${root}/board/tags/${bug.id}`,
      null,
      200,
    );

    expect(one.tagsRequired).toBe(true);

    const none = await expectStatus<BoardTagCatalog>(
      server,
      "DELETE",
      `${root}/board/tags/${idea.id}`,
      null,
      200,
    );

    expect([none.tagsRequired, none.defaultBoardTagId]).toEqual([false, null]);
    await expectStatus<ThreadDetail>(server, "POST", `${root}/posts`, post([]), 201);

    const refused = await send(server, "PATCH", `${root}/board`, {
      tagsRequired: true,
      defaultBoardTagId: null,
    });

    expect(fieldsOf(refused)).toEqual({ tags_required: ["needs at least one board tag"] });
  });

  it("matches names and filters under full case folding", async () => {
    const { server } = harness();

    await seedCatalog(server, { tags: [{ name: "Straße" }] });

    const made = await expectStatus<ThreadDetail>(
      server,
      "POST",
      `${root}/posts`,
      post(["STRASSE"]),
      201,
    );

    expect(made.thread.work?.tags).toEqual(["Straße"]);

    const filtered = await get<BoardListing>(server, `${root}/board?tag=strasse`);

    expect(filtered.tag).toBe("Straße");
    expect(filtered.posts.map(({ thread }) => thread.id)).toEqual([made.thread.id]);
    expect(fieldsOf(await send(server, "POST", `${root}/board/tags`, { name: "STRASSE" }))).toEqual(
      { name: ["has already been taken"] },
    );
  });

  it("filters by a tag the runtime lower-cases but the server's folding keeps (U+A7CE)", async () => {
    // As a Unicode 17 runtime does; the server (caseless, Unicode 16) keeps U+A7CE as it is.
    const lower = String.prototype.toLowerCase;

    vi.spyOn(String.prototype, "toLowerCase").mockImplementation(function (this: string) {
      return lower.call(this).replaceAll("\u{a7ce}", "\u{a7cf}");
    });

    const { server } = harness();

    await seedCatalog(server, { tags: [{ name: "\u{a7ce}" }] });

    const made = await expectStatus<ThreadDetail>(
      server,
      "POST",
      `${root}/posts`,
      post(["\u{a7ce}"]),
      201,
    );

    const filtered = await get<BoardListing>(server, `${root}/board?tag=%EA%9F%8E`);

    expect(filtered.tag).toBe("\u{a7ce}");
    expect(filtered.posts.map(({ thread }) => thread.id)).toEqual([made.thread.id]);
  });
});

describe("mock board tag authorization", () => {
  function permissionHarness() {
    const world = buildWorld(NOW, 1);
    const board = world.rooms.get(BOARD_ROOM_ID);
    const viewer = world.users.get(VIEWER_ID);

    if (board === undefined || viewer === undefined) throw new Error("Seed missing");

    const { routes } = createBoardTags({
      world: () => world,
      publish: () => undefined,
      roomOr404: (roomId) => {
        const room = world.rooms.get(roomId);

        if (room === undefined) throw notFound();

        return room;
      },
    });

    const request = async (method: string, path: string, body: Json = null) => {
      const handler = dispatch(routes, method, path, new URLSearchParams(), body);

      if (handler === null) throw new Error("Route missing");

      try {
        return await handler();
      } catch (error) {
        if (error instanceof HttpError) return { status: error.status, json: null };

        throw error;
      }
    };

    return { board, viewer, request };
  }

  it.each([
    { method: "POST", suffix: "/board/tags", body: { name: "Bug" } },
    { method: "PUT", suffix: "/board/tags/order", body: { tagIds: [] } },
    { method: "PATCH", suffix: "/board", body: { tagsRequired: true, defaultBoardTagId: null } },
  ])("lets members read but not $method $suffix", async ({ method, suffix, body }) => {
    const { board, viewer, request } = permissionHarness();

    viewer.role = "member";
    board.room.creatorId = USER_IDS.maya;

    expect((await request("GET", `/rooms/${BOARD_ROOM_ID}/board/tags`)).status).toBe(200);
    expect((await request(method, `/rooms/${BOARD_ROOM_ID}${suffix}`, body)).status).toBe(403);
    expect((await request("GET", "/rooms/1/board/tags")).status).toBe(404);
  });
});
