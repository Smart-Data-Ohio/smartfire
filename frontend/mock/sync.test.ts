import { describe, expect, it } from "vitest";
import type { MessageDTO } from "../src/gen/MessageDTO.ts";
import type { ServerFrame } from "../src/gen/ServerFrame.ts";
import type { SyncEvent } from "../src/gen/SyncEvent.ts";
import { type ManualScheduler, manualScheduler } from "./scheduler.ts";
import { createMockServer, type MockServer, SEED_IDS } from "./server.ts";
import { BOT_STREAM_STEP_MS, BOT_TYPING_DELAY_MS, BOT_TYPING_MS } from "./simulation.ts";
import { PING_AFTER_MS, parseClientFrame, RING_SIZE, type SyncConnection } from "./sync.ts";

const NOW = Date.UTC(2026, 9, 6, 16, 30, 0);

const { rooms, users } = SEED_IDS;

interface Client {
  readonly frames: ServerFrame[];
  readonly connection: SyncConnection;
  events(): SyncEvent[];
  dropped(): boolean;
}

interface Harness {
  readonly server: MockServer;
  readonly clock: ManualScheduler;
}

function setup(simulate = false): Harness {
  const clock = manualScheduler(NOW);

  return { server: createMockServer({ now: clock.now, scheduler: clock, simulate }), clock };
}

function open(server: MockServer, topics: string[], resume: Client | null = null): Client {
  const frames: ServerFrame[] = [];
  let dropped = false;

  const connection = server.connect(
    (frame) => frames.push(frame),
    () => {
      dropped = true;
    },
  );

  const welcome = resume?.frames.find((frame) => frame.t === "welcome");
  const lastSeq = resume?.events().at(-1)?.seq;

  connection.receive({
    t: "hello",
    v: 1,
    resume: welcome?.t === "welcome" ? { epoch: welcome.epoch, seq: lastSeq ?? welcome.seq } : null,
    topics,
  });

  return {
    frames,
    connection,
    events: () => frames.flatMap((frame) => (frame.t === "batch" ? frame.events : [])),
    dropped: () => dropped,
  };
}

describe("hello and resume", () => {
  it("welcomes a fresh client at the current sequence, not resumed", () => {
    const { server } = setup();

    server.post(rooms.general, users.maya, "before anyone connected");

    const client = open(server, []);

    expect(client.frames).toEqual([
      {
        t: "welcome",
        epoch: server.syncState().epoch,
        seq: server.syncState().seq,
        resumed: false,
      },
    ]);
  });

  it("resumes within the ring and replays only what was missed on its topics", () => {
    const { server } = setup();
    const first = open(server, [`room:${rooms.general}`]);

    server.post(rooms.general, users.maya, "seen live");
    first.connection.close();

    const missed = server.post(rooms.general, users.jonah, "missed while away");

    server.post(rooms.design, users.lucia, "not my topic");

    const second = open(server, [`room:${rooms.general}`], first);
    const [welcome, replay] = second.frames;

    expect(welcome).toMatchObject({ t: "welcome", resumed: true });
    expect(replay?.t).toBe("batch");

    const replayed = second.events();
    const created = replayed.filter((event) => event.type === "message.created");

    expect(created.map((event) => event.data)).toEqual([missed]);
    expect(
      replayed.every((event) => event.topic === "user" || event.topic === `room:${rooms.general}`),
    ).toBe(true);
    expect(replayed.every((event) => event.seq > (first.events().at(-1)?.seq ?? 0))).toBe(true);
  });

  it("asks for a resync (resumed: false) after the ring rolls over or the epoch changes", () => {
    const { server } = setup();
    const first = open(server, []);

    first.connection.close();

    for (let index = 0; index <= RING_SIZE; index++) server.typing(rooms.general, users.maya, true);

    expect(open(server, [], first).frames[0]).toMatchObject({ t: "welcome", resumed: false });

    const recent = open(server, []);

    recent.connection.close();
    server.reset();
    expect(recent.dropped()).toBe(false);
    expect(open(server, [], recent).frames[0]).toMatchObject({ t: "welcome", resumed: false });
  });
});

describe("topics", () => {
  it("sends events only to connections subscribed to their topic", () => {
    const { server } = setup();
    const general = open(server, [`room:${rooms.general}`]);
    const design = open(server, [`room:${rooms.design}`]);

    server.typing(rooms.general, users.maya, true);
    expect(general.events().map((event) => event.topic)).toEqual([`room:${rooms.general}`]);
    expect(design.events()).toEqual([]);

    design.connection.receive({ t: "sub", topics: [`room:${rooms.general}`] });
    general.connection.receive({ t: "unsub", topics: [`room:${rooms.general}`] });
    server.typing(rooms.general, users.maya, false);
    expect(general.events()).toHaveLength(1);
    expect(design.events()).toHaveLength(1);

    server.setPresence(users.sam, "online");
    expect(general.events().at(-1)?.topic).toBe("user");
    expect(design.events().at(-1)?.topic).toBe("user");
  });

  it("fans typing out to other connections, never back to the typist", () => {
    const { server } = setup();
    const typist = open(server, [`room:${rooms.general}`]);
    const watcher = open(server, [`room:${rooms.general}`]);

    typist.connection.receive({ t: "typing", conv: `room:${rooms.general}`, on: true });

    expect(typist.events()).toEqual([]);
    expect(watcher.events()).toEqual([
      expect.objectContaining({
        topic: `room:${rooms.general}`,
        type: "typing",
        data: { userId: SEED_IDS.viewer, on: true },
      }),
    ]);
  });

  it("gives every event a strictly increasing sequence number", () => {
    const { server } = setup();
    const client = open(server, [`room:${rooms.general}`]);

    server.typing(rooms.general, users.maya, true);
    server.post(rooms.general, users.maya, "one");
    server.post(rooms.general, users.maya, "two");

    const seqs = client.events().map((event) => event.seq);

    expect(seqs).toEqual([...seqs].sort((a, b) => a - b));
    expect(new Set(seqs).size).toBe(seqs.length);
  });
});

describe("connection lifecycle", () => {
  it("pings after 15 s without other traffic", () => {
    const { server, clock } = setup();
    const client = open(server, []);

    clock.advance(PING_AFTER_MS - 1);
    expect(client.frames.at(-1)?.t).toBe("welcome");

    clock.advance(1);
    expect(client.frames.at(-1)).toEqual({ t: "ping" });

    clock.advance(PING_AFTER_MS);
    expect(client.frames.filter((frame) => frame.t === "ping")).toHaveLength(2);
  });

  it("drops connections abruptly or with a bye", () => {
    const { server } = setup();
    const abrupt = open(server, []);

    server.dropConnections();
    expect(abrupt.dropped()).toBe(true);
    expect(abrupt.frames.at(-1)?.t).toBe("welcome");

    const polite = open(server, []);

    server.dropConnections(true);
    expect(polite.frames.at(-1)).toEqual({ t: "bye", reconnect: true, reason: "server_restart" });
    expect(server.syncState().connections).toBe(0);
  });

  it("parses client frames and rejects anything else", () => {
    expect(parseClientFrame({ t: "hello", v: 1, resume: null, topics: ["room:1"] })).toEqual({
      t: "hello",
      v: 1,
      resume: null,
      topics: ["room:1"],
    });
    expect(parseClientFrame({ t: "present", room: 3 })).toEqual({ t: "present", room: 3 });
    expect(parseClientFrame({ t: "typing", conv: "room:1" })).toBeNull();
    expect(parseClientFrame({ t: "nope" })).toBeNull();
    expect(parseClientFrame("hello")).toBeNull();
  });
});

describe("the bot", () => {
  it("types, then streams its reply as message.created and growing message.updated", async () => {
    const { server, clock } = setup(true);

    server.pause();

    const client = open(server, [`room:${rooms.dmEmber}`]);

    const response = await server.handle({
      method: "POST",
      path: `/api/v1/rooms/${rooms.dmEmber}/messages`,
      body: {
        clientMessageId: "to-ember",
        markdownSource: "How is CI?",
        replyToMessageId: null,
        replyNotifyAuthor: null,
      },
      headers: { "x-csrf-token": server.csrfToken() },
    });

    expect(response.status).toBe(201);
    clock.advance(BOT_TYPING_DELAY_MS + BOT_TYPING_MS + 10 * BOT_STREAM_STEP_MS);

    const roomEvents = client.events().filter((event) => event.topic === `room:${rooms.dmEmber}`);

    const steps = roomEvents.map((event) =>
      event.type === "typing" ? `typing:${event.data.on}` : event.type,
    );

    expect(steps.slice(0, 4)).toEqual([
      "message.created",
      "typing:true",
      "typing:false",
      "message.created",
    ]);

    const updates = steps.slice(4);

    expect(updates.length).toBeGreaterThanOrEqual(3);
    expect(updates.length).toBeLessThanOrEqual(5);
    expect(new Set(updates)).toEqual(new Set(["message.updated"]));

    const bodies: MessageDTO[] = [];

    for (const event of roomEvents.slice(3)) {
      if (event.type === "message.created" || event.type === "message.updated")
        bodies.push(event.data);
    }

    expect(bodies.every((message) => message.creatorId === SEED_IDS.bot)).toBe(true);
    expect(new Set(bodies.map((message) => message.id)).size).toBe(1);
    expect(bodies.map((message) => message.streaming)).toEqual([
      ...bodies.slice(0, -1).map(() => true),
      false,
    ]);

    for (let index = 1; index < bodies.length; index++) {
      const previous = bodies[index - 1];
      const current = bodies[index];

      expect(current?.bodyHtml.length).toBeGreaterThan(previous?.bodyHtml.length ?? 0);
      expect((current?.updatedAt ?? "") > (previous?.updatedAt ?? "")).toBe(true);
    }
  });

  it("stays quiet when the simulation is off", async () => {
    const { server, clock } = setup(false);
    const client = open(server, [`room:${rooms.dmEmber}`]);

    server.post(rooms.dmEmber, SEED_IDS.viewer, "hello?");
    clock.advance(60_000);

    expect(client.events().filter((event) => event.type === "message.created")).toHaveLength(1);
  });
});

describe("the ambient simulation", () => {
  it("has someone type and then post every 6 to 20 seconds, and pauses", () => {
    const { server, clock } = setup(true);
    const client = open(server, [`room:${rooms.general}`]);

    clock.advance(25_000);

    // Every post from someone else updates its sidebar row on the user topic, wherever it lands.
    const posts = client.events().filter((event) => event.type === "sidebar.row.upserted");

    expect(posts.length).toBeGreaterThanOrEqual(1);
    expect(client.events().some((event) => event.type === "typing")).toBe(true);

    server.pause();
    clock.advance(10_000);

    const paused = client.events().length;

    clock.advance(120_000);
    expect(client.events().length - paused).toBe(0);
  });
});
