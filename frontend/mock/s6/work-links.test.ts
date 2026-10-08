import { describe, expect, it } from "vitest";
import type { ThreadDetail } from "../../src/gen/ThreadDetail.ts";
import type { WorkLinkForm } from "../../src/gen/WorkLinkForm.ts";
import { boardDetail } from "../../src/test/board-fixtures.ts";
import { notFound } from "../http.ts";
import { field } from "../json.ts";
import { dispatch } from "../s2/context.ts";
import { threadDto } from "../s2/model.ts";
import { collect, errorOf, expectStatus, get, harness, NOW, send } from "../s2/testing.ts";
import { buildWorld } from "../s3/seed.ts";
import { USER_IDS, VIEWER_ID } from "../seed.ts";
import { BOARD_ROOM_ID } from "./seed.ts";
import { createWorkLinks } from "./work-links.ts";

const path = "/api/v1/threads/9001/work/links";

const pr = "https://github.com/Smartfire/Test/pulls/00123/files";

const drive = "https://docs.google.com/document/d/1234567890/edit";

describe("mock work link routes", () => {
  it("offers Roadmap events, creates each kind, publishes once and installs full details", async () => {
    const { server } = harness();
    const events = collect(server, [`room:${BOARD_ROOM_ID}`, "thread:9001"]);
    const form = await get<WorkLinkForm>(server, `${path}/new`);
    expect(form.events.map(({ id }) => id)).toEqual([9104, 9105, 9106]);
    expect(form.events[0]).toMatchObject({ title: "API review", timeZone: "America/New_York" });

    for (const input of [
      { kind: "pull_request", pullRequestUrl: pr },
      { kind: "event", eventId: 9104 },
      { kind: "drive_file", driveUrl: ` ${drive}\n` },
    ]) {
      const saved = await expectStatus<ThreadDetail>(server, "POST", path, input, 201);
      expect(saved.thread.work?.links.at(-1)?.kind).toBe(input.kind);
    }

    const saved = await get<ThreadDetail>(server, "/api/v1/threads/9001");
    expect(saved.thread.work?.links.map(({ url }) => url)).toEqual([
      "https://github.com/smartfire/test/pull/123",
      "/rooms/900/events/9104",
      drive,
    ]);
    expect((await get<WorkLinkForm>(server, `${path}/new`)).events.map(({ id }) => id)).toEqual([
      9105, 9106,
    ]);
    const updates = events.filter(({ type }) => type === "thread.updated");
    expect(updates).toHaveLength(6);
    expect(updates.filter(({ topic }) => topic === "thread:9001")).toHaveLength(3);
    expect(updates.filter(({ topic }) => topic === `room:${BOARD_ROOM_ID}`)).toHaveLength(3);
    expect(field(updates[5]?.data, "work")).toEqual(saved.thread.work);
    const id = saved.thread.work?.links[1]?.id;
    const removed = await expectStatus<ThreadDetail>(server, "DELETE", `${path}/${id}`, null, 200);
    expect(removed.thread.work?.links).toHaveLength(2);
    expect(events.filter(({ type }) => type === "thread.updated")).toHaveLength(8);
    expect((await get<WorkLinkForm>(server, `${path}/new`)).events.map(({ id }) => id)).toEqual([
      9104, 9105, 9106,
    ]);
    server.dispose();
  });

  it("preserves every classic prompt, duplicates and missing/foreign IDs", async () => {
    const { server } = harness();

    const cases = [
      [
        { kind: "pull_request", pullRequestUrl: "http://github.com/a/b/pull/1" },
        "pullRequestUrl",
        "Enter a GitHub pull request URL, like https://github.com/owner/repo/pull/123.",
      ],
      [{ kind: "event" }, "eventId", "Choose an event to link."],
      [{ kind: "event", eventId: null }, "eventId", "Choose an event to link."],
      [
        { kind: "drive_file", driveUrl: "https://example.com/file" },
        "driveUrl",
        "Enter a Google Drive, Docs, Sheets, Slides, or Forms link.",
      ],
      [{ kind: "other" }, "kind", "Choose a pull request, event, or Drive file to link."],
      [{}, "kind", "Choose a pull request, event, or Drive file to link."],
    ] as const;

    for (const [input, key, message] of cases) {
      const response = await send(server, "POST", path, input);
      expect(response.status).toBe(422);
      expect(errorOf(response.json)).toEqual({ tag: "Validation", message });
      expect(field(field(response.json, "error"), "fields")).toEqual({ [key]: [message] });
    }

    for (const [input, key] of [
      [{ kind: "pull_request", pullRequestUrl: pr }, "pullRequestUrl"],
      [{ kind: "event", eventId: 9104 }, "eventId"],
      [{ kind: "drive_file", driveUrl: drive }, "driveUrl"],
    ] as const) {
      await expectStatus<ThreadDetail>(server, "POST", path, input, 201);
      const response = await send(server, "POST", path, input);
      const message = "That is already linked to this work thread.";
      expect(response.status).toBe(422);
      expect(errorOf(response.json)).toEqual({ tag: "Validation", message });
      expect(field(field(response.json, "error"), "fields")).toEqual({ [key]: [message] });
    }

    expect(
      (
        await send(server, "POST", path, {
          kind: "pull_request",
          pullRequestUrl: "https://github.com/smartfire/test/pull/123",
        })
      ).status,
    ).toBe(422);

    const other = await expectStatus<ThreadDetail>(
      server,
      "POST",
      "/api/v1/threads/9002/work/links",
      { kind: "event", eventId: 9105 },
      201,
    );

    expect(
      (await send(server, "DELETE", `${path}/${other.thread.work?.links[0]?.id}`)).status,
    ).toBe(404);
    expect((await send(server, "DELETE", `${path}/9999999`)).status).toBe(404);
    expect((await send(server, "POST", path, { kind: "event", eventId: 9999999 })).status).toBe(
      404,
    );

    const tracked = await send(server, "POST", "/api/v1/threads/1/work/links", {
      kind: "event",
      eventId: 9104,
    });

    expect(tracked.status).toBe(422);
    expect(errorOf(tracked.json).message).toBe("This thread isn't tracked as work");
    expect((await send(server, "POST", "/api/v1/threads/9999999/work/links", {})).status).toBe(404);
    server.dispose();
  });
});

describe("mock work link scope and picker parity", () => {
  function setup() {
    const world = buildWorld(NOW, 1);

    const routes = createWorkLinks(
      { world: () => world, now: () => NOW, publish: () => {} },
      {
        threadOr404(id) {
          const thread = world.threads.get(id);

          if (thread === undefined) throw notFound();

          return thread;
        },
        detail(thread) {
          return boardDetail(threadDto(thread, NOW));
        },
      },
    ).routes;

    const request = (
      method: string,
      url: string,
      body: Parameters<typeof dispatch>[4] = undefined,
    ) => {
      const handler = dispatch(routes, method, url, new URLSearchParams(), body);

      if (handler === null) throw new Error("Route missing");

      return handler;
    };

    return { world, request };
  }

  it("lets an ordinary viewer manage links and rejects nonmembers/inactive/bot viewers", async () => {
    const { world, request } = setup();
    const viewer = world.users.get(VIEWER_ID);
    const room = world.rooms.get(BOARD_ROOM_ID);

    if (viewer === undefined || room === undefined) throw new Error("Seed missing");
    viewer.role = "member";
    room.room.creatorId = USER_IDS.priya;
    expect(
      (await request("POST", "/threads/9002/work/links", { kind: "event", eventId: 9104 })())
        .status,
    ).toBe(201);
    room.memberIds = room.memberIds.filter((id) => id !== VIEWER_ID);
    expect(request("GET", "/threads/9002/work/links/new")).toThrowError("Not found");
    room.memberIds.push(VIEWER_ID);
    viewer.status = "deactivated";
    expect(request("GET", "/threads/9002/work/links/new")).toThrowError("Not found");
    viewer.status = "active";
    viewer.role = "bot";
    expect(request("DELETE", "/threads/9002/work/links/1")).toThrowError("Not found");
  });

  it("filters ended/cancelled/foreign/linked candidates, orders ties, accepts past room events", async () => {
    const { world, request } = setup();
    const seed = world.workLinkEvents.get(9104);

    if (seed === undefined) throw new Error("Event missing");
    world.workLinkEvents.set(9200, {
      ...seed,
      id: 9200,
      title: "Ongoing",
      startsAt: new Date(NOW - 1000).toISOString(),
      endsAt: new Date(NOW + 1000).toISOString(),
    });
    world.workLinkEvents.set(9201, {
      ...seed,
      id: 9201,
      title: "Past",
      startsAt: new Date(NOW - 1000).toISOString(),
      endsAt: null,
    });
    world.workLinkEvents.set(9202, { ...seed, id: 9202, title: "Cancelled", cancelled: true });
    world.workLinkEvents.set(9203, { ...seed, id: 9203, title: "Foreign", roomId: 1 });
    world.workLinkEvents.set(9204, { ...seed, id: 9204, title: "Same start" });
    request("POST", "/threads/9002/work/links", { kind: "event", eventId: 9104 })();
    const response = await request("GET", "/threads/9002/work/links/new")();
    expect(field(response.json, "events")).toEqual(
      [
        world.workLinkEvents.get(9200),
        world.workLinkEvents.get(9204),
        world.workLinkEvents.get(9105),
        world.workLinkEvents.get(9106),
      ].map((event) => ({
        id: event?.id,
        title: event?.title,
        startsAt: event?.startsAt,
        timeZone: event?.timeZone,
      })),
    );
    expect(
      request("POST", "/threads/9002/work/links", { kind: "event", eventId: 9203 }),
    ).toThrowError("Not found");
    expect(
      (await request("POST", "/threads/9002/work/links", { kind: "event", eventId: 9201 })())
        .status,
    ).toBe(201);
    expect(
      (await request("POST", "/threads/9002/work/links", { kind: "event", eventId: 9202 })())
        .status,
    ).toBe(201);
  });
});
