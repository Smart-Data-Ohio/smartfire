/**
 * Huddles, voice rooms and stages for the mock backend: who's in which call, joining (with a
 * `mock://` URL and a JSON token the client's fake transport reads), leaving, moderation, and a
 * stage's roster, raised hands and stream. Every change goes out as the real server sends it:
 * `huddle.presence` (and `huddle.notice`) on the `user` topic, `stage.updated` on `room:<id>`,
 * `huddle.role` and `stage.stream.stopped` on the affected person's `user` topic (only the
 * viewer's reaches this tab). Tests steer the other people through `control`.
 */
import type { HuddleCredentials } from "../../src/gen/HuddleCredentials.ts";
import type { HuddleDetail } from "../../src/gen/HuddleDetail.ts";
import type { HuddleModeration } from "../../src/gen/HuddleModeration.ts";
import type { HuddleParticipant } from "../../src/gen/HuddleParticipant.ts";
import type { HuddlePresence } from "../../src/gen/HuddlePresence.ts";
import type { HuddlePresenceList } from "../../src/gen/HuddlePresenceList.ts";
import type { HuddleRingEvent } from "../../src/gen/HuddleRingEvent.ts";
import type { HuddleRingState } from "../../src/gen/HuddleRingState.ts";
import type { StageDetail } from "../../src/gen/StageDetail.ts";
import type { StageMember } from "../../src/gen/StageMember.ts";
import type { StageRole } from "../../src/gen/StageRole.ts";
import type { StageState } from "../../src/gen/StageState.ts";
import type { StageStream } from "../../src/gen/StageStream.ts";
import type { StreamQuality } from "../../src/gen/StreamQuality.ts";
import {
  conflict,
  forbidden,
  type MockResponse,
  noContent,
  notFound,
  validation,
} from "../http.ts";
import { intField, stringField } from "../json.ts";
import { firstId, type Route, route, type S2Context } from "../s2/context.ts";
import { type RoomRecord, USER_IDS, VIEWER_ID, type World } from "../seed.ts";
import type { Outgoing } from "../sync.ts";

/** The mock's LiveKit URL: the client swaps in its fake transport for it. */
export const MOCK_LIVEKIT_URL = "mock://livekit";

const STAGE_ROLES: readonly StageRole[] = ["listener", "speaker", "host"];

const QUALITIES: readonly StreamQuality[] = ["720p15", "1080p15", "1080p30"];

const MODERATIONS: readonly HuddleModeration[] = ["mute", "unmute", "disconnect"];

/** Someone in a call: one LiveKit identity per tab, oldest first. */
interface CallEntry {
  identities: string[];
}

interface StageRecord {
  /** Roles by user id (every member has one; listeners by default). */
  readonly roles: Map<number, StageRole>;
  /** Raised hands by user id, as ISO timestamps. */
  readonly hands: Map<number, string>;
  live: StageStream | null;
}

/** The calls, per world (a reset starts over with the new world). */
interface CallWorld {
  /** Room id → user id → their entry, in join order. */
  readonly calls: Map<number, Map<number, CallEntry>>;
  /** `${roomId}:${userId}` for everyone a host muted. */
  readonly muted: Set<string>;
  readonly stages: Map<number, StageRecord>;
  nextGrantId: number;
  nextStreamId: number;
}

/** Controls the server forwards (`/__mock/<action>`), with its field readers. */
export interface HuddleControlInput {
  readonly int: (key: string) => number;
  readonly text: (key: string) => string;
  readonly flag: (key: string, fallback: boolean) => boolean;
  readonly optionalText: (key: string) => string | null;
}

export interface Huddles {
  readonly routes: readonly Route[];
  /** Answers a huddle control, or `null` when the action isn't one of these. */
  control(action: string, input: HuddleControlInput): MockResponse | null;
}

/** Whether a room kind holds calls (boards don't). */
function callable(record: RoomRecord): boolean {
  return record.room.kind !== "board";
}

const byName = (names: ReadonlyMap<number, string>) => (a: number, b: number) => {
  const [left, right] = [(names.get(a) ?? "").toLowerCase(), (names.get(b) ?? "").toLowerCase()];

  return left < right ? -1 : left > right ? 1 : a - b;
};

export function createHuddles(ctx: S2Context, simulate: boolean): Huddles {
  const worlds = new WeakMap<World, CallWorld>();

  const calls = (): CallWorld => {
    const world = ctx.world();
    let state = worlds.get(world);

    if (state === undefined) {
      state = seedCalls(world);
      worlds.set(world, state);
    }

    return state;
  };

  const identity = () => `campfire-participant-${ctx.hex(12)}`;

  /** A seeded starting point: Maya and Jonah are chatting in the Lounge. */
  function seedCalls(world: World): CallWorld {
    const state: CallWorld = {
      calls: new Map(),
      muted: new Set(),
      stages: new Map(),
      nextGrantId: 1,
      nextStreamId: 1,
    };

    const lounge = world.rooms.get(8);

    if (lounge !== undefined) {
      state.calls.set(
        lounge.room.id,
        new Map([
          [USER_IDS.maya, { identities: [`campfire-participant-maya-${lounge.room.id}`] }],
          [USER_IDS.jonah, { identities: [`campfire-participant-jonah-${lounge.room.id}`] }],
        ]),
      );
    }

    return state;
  }

  /** The room's record if it exists at all (the viewer may not belong to it). */
  const recordOf = (roomId: number): RoomRecord => {
    const record = ctx.world().rooms.get(roomId);

    if (record === undefined) throw notFound("Room not found");

    return record;
  };

  const callRoom = (roomId: number): RoomRecord => {
    const record = ctx.roomOr404(roomId);

    if (!callable(record)) throw notFound("Room not found");

    return record;
  };

  const stageRoom = (roomId: number): RoomRecord => {
    const record = ctx.roomOr404(roomId);

    if (record.room.kind !== "stage") throw notFound("Room not found");

    return record;
  };

  const membershipId = (record: RoomRecord, userId: number): number =>
    userId === VIEWER_ID ? record.membership.id : 50_000 + record.room.id * 100 + userId;

  const userForMembership = (record: RoomRecord, id: number): number | null => {
    for (const userId of record.memberIds) {
      if (membershipId(record, userId) === id) return userId;
    }

    return null;
  };

  const isAdmin = (userId: number) => ctx.world().users.get(userId)?.role === "administrator";

  const nameOf = (userId: number) => ctx.world().users.get(userId)?.name ?? "Someone";

  // --- stages ---

  const stageOf = (record: RoomRecord): StageRecord => {
    const state = calls();
    let stage = state.stages.get(record.room.id);

    if (stage === undefined) {
      stage = { roles: new Map(), hands: new Map(), live: null };

      for (const userId of record.memberIds) {
        const seeded =
          userId === VIEWER_ID
            ? (record.membership.stageRole ?? "listener")
            : userId === USER_IDS.priya
              ? "speaker"
              : "listener";

        stage.roles.set(userId, seeded);
      }

      state.stages.set(record.room.id, stage);
    }

    return stage;
  };

  const roleOf = (record: RoomRecord, userId: number): StageRole | null =>
    record.room.kind === "stage" ? (stageOf(record).roles.get(userId) ?? "listener") : null;

  const stageState = (record: RoomRecord): StageState => {
    const stage = stageOf(record);
    const muted = calls().muted;
    const names = new Map(record.memberIds.map((id) => [id, nameOf(id)]));
    const order = byName(names);

    const rank = (userId: number): number =>
      STAGE_ROLES.length - 1 - STAGE_ROLES.indexOf(stage.roles.get(userId) ?? "listener");

    const members = [...record.memberIds].sort((a, b) => {
      const byRank = rank(a) - rank(b);

      if (byRank !== 0) return byRank;

      const [handA, handB] = [stage.hands.get(a), stage.hands.get(b)];

      if (handA !== undefined && handB !== undefined && handA !== handB) {
        return handA < handB ? -1 : 1;
      }

      if (handA !== undefined && handB === undefined) return -1;

      if (handB !== undefined && handA === undefined) return 1;

      return order(a, b);
    });

    return {
      roomId: record.room.id,
      members: members.map(
        (userId): StageMember => ({
          membershipId: membershipId(record, userId),
          userId,
          role: stage.roles.get(userId) ?? "listener",
          handRaisedAt: stage.hands.get(userId) ?? null,
          serverMuted: muted.has(`${record.room.id}:${userId}`),
        }),
      ),
      live: stage.live,
    };
  };

  const stageEvent = (record: RoomRecord): Outgoing => ({
    topic: `room:${record.room.id}`,
    type: "stage.updated",
    data: stageState(record),
  });

  // --- presence ---

  const presenceOf = (record: RoomRecord): HuddlePresence => {
    const state = calls();
    const entries = state.calls.get(record.room.id) ?? new Map<number, CallEntry>();
    const names = new Map([...entries.keys()].map((id) => [id, nameOf(id)]));

    const participants = [...entries.keys()].sort(byName(names)).map(
      (userId): HuddleParticipant => ({
        userId,
        membershipId: membershipId(record, userId),
        identities: [...(entries.get(userId)?.identities ?? [])],
        serverMuted: state.muted.has(`${record.room.id}:${userId}`),
      }),
    );

    return {
      roomId: record.room.id,
      participants,
      live: record.room.kind === "stage" && stageOf(record).live !== null,
    };
  };

  const presenceEvent = (record: RoomRecord): Outgoing => ({
    topic: "user",
    type: "huddle.presence",
    data: presenceOf(record),
  });

  const inCall = (roomId: number, userId: number): boolean =>
    calls().calls.get(roomId)?.has(userId) ?? false;

  /** Ends the room's stream if its presenter is no longer allowed to present. */
  const endStreamIfOrphaned = (record: RoomRecord, events: Outgoing[]): void => {
    if (record.room.kind !== "stage") return;

    const stage = stageOf(record);
    const live = stage.live;

    if (live === null) return;

    const role = stage.roles.get(live.userId);
    const muted = calls().muted.has(`${record.room.id}:${live.userId}`);

    if (inCall(record.room.id, live.userId) && role !== "listener" && !muted) return;

    stage.live = null;
    events.push(stageEvent(record));
  };

  /** Puts someone in a call (another tab of theirs adds an identity). */
  const enter = (record: RoomRecord, userId: number, newIdentity: string, replace: boolean) => {
    const state = calls();
    const roomId = record.room.id;
    let entries = state.calls.get(roomId);

    if (entries === undefined) {
      entries = new Map();
      state.calls.set(roomId, entries);
    }

    const rejoin = entries.has(userId);
    const entry = entries.get(userId) ?? { identities: [] };

    entry.identities = replace ? [newIdentity] : [...entry.identities, newIdentity];
    entries.set(userId, entry);

    const events: Outgoing[] = [presenceEvent(record)];

    if (userId !== VIEWER_ID && !rejoin) {
      events.push({
        topic: "user",
        type: "huddle.notice",
        data: {
          kind: "joined",
          roomId,
          roomName: ctx.displayName(record),
          userId,
          userName: nameOf(userId),
          inCall: inCall(roomId, VIEWER_ID),
          rejoin: false,
        },
      });
    }

    if (record.room.kind === "stage") events.push(stageEvent(record));

    ctx.publish(events);
  };

  /** Takes someone out of a call; the last one out ends it. */
  const exit = (record: RoomRecord, userId: number): boolean => {
    const state = calls();
    const roomId = record.room.id;
    const entries = state.calls.get(roomId);

    if (entries?.delete(userId) !== true) return false;

    if (entries.size === 0) state.calls.delete(roomId);

    const events: Outgoing[] = [];

    endStreamIfOrphaned(record, events);
    events.unshift(presenceEvent(record));

    if (userId !== VIEWER_ID) {
      events.push({
        topic: "user",
        type: "huddle.notice",
        data:
          entries.size === 0
            ? { kind: "ended", roomId }
            : {
                kind: "left",
                roomId,
                roomName: ctx.displayName(record),
                userId,
                userName: nameOf(userId),
              },
      });
    }

    ctx.publish(events);

    return true;
  };

  /** The viewer's grant was revoked and reissued (a role change or a mute). */
  const roleChanged = (record: RoomRecord, userId: number): Outgoing[] =>
    userId === VIEWER_ID
      ? [
          {
            topic: "user",
            type: "huddle.role",
            data: {
              roomId: record.room.id,
              stageRole: roleOf(record, userId),
              serverMuted: calls().muted.has(`${record.room.id}:${userId}`),
            },
          },
        ]
      : [];

  const canPublish = (record: RoomRecord, userId: number): boolean =>
    !calls().muted.has(`${record.room.id}:${userId}`) && roleOf(record, userId) !== "listener";

  const canManageStage = (record: RoomRecord): boolean =>
    isAdmin(VIEWER_ID) || roleOf(record, VIEWER_ID) === "host";

  // --- endpoints ---

  const list = (): HuddlePresenceList => {
    const rooms: HuddlePresence[] = [];
    const people = new Set<number>();

    for (const roomId of calls().calls.keys()) {
      const record = ctx.world().rooms.get(roomId);

      if (record === undefined || record.membership.involvement === "invisible") continue;

      const presence = presenceOf(record);

      rooms.push(presence);

      for (const participant of presence.participants) people.add(participant.userId);
    }

    return { rooms, users: ctx.usersFor(people) };
  };

  const detail = (roomId: number): HuddleDetail => {
    const record = callRoom(roomId);
    const presence = presenceOf(record);

    return {
      roomName: ctx.displayName(record),
      presence,
      users: ctx.usersFor(presence.participants.map((participant) => participant.userId)),
    };
  };

  const join = (roomId: number): HuddleCredentials => {
    const record = callRoom(roomId);
    const state = calls();
    const grantId = state.nextGrantId++;
    const newIdentity = identity();

    // One call at a time per tab: the gateway notices the old connection drop within a second.
    for (const [otherId, entries] of state.calls) {
      if (otherId !== roomId && entries.has(VIEWER_ID)) exit(recordOf(otherId), VIEWER_ID);
    }

    enter(record, VIEWER_ID, newIdentity, true);

    const publish = canPublish(record, VIEWER_ID);

    return {
      url: MOCK_LIVEKIT_URL,
      token: JSON.stringify({
        identity: newIdentity,
        userId: VIEWER_ID,
        canPublish: publish,
        simulate,
      }),
      identity: newIdentity,
      grantId,
      roomId,
      roomName: ctx.displayName(record),
      canPublish: publish,
    };
  };

  const leave = (roomId: number): void => {
    exit(callRoom(roomId), VIEWER_ID);
  };

  const moderate = (roomId: number, targetId: number, action: HuddleModeration): void => {
    const record = callRoom(roomId);

    // Moderation is a voice and stage room thing; elsewhere the endpoint doesn't exist.
    if (record.room.kind !== "voice" && record.room.kind !== "stage") throw notFound("Not found");

    if (!(isAdmin(VIEWER_ID) || roleOf(record, VIEWER_ID) === "host")) {
      throw forbidden("Only administrators and stage hosts can moderate calls");
    }

    const userId = userForMembership(record, targetId);

    if (userId === null) throw notFound("Not found");

    // An administrator may lift their own mute; nothing else targets oneself.
    if (userId === VIEWER_ID && !(action === "unmute" && isAdmin(VIEWER_ID))) {
      throw validation("membershipId", "You cannot moderate your own call session");
    }

    if (isAdmin(userId) && !isAdmin(VIEWER_ID)) {
      throw forbidden("Only administrators can moderate an administrator");
    }

    if (action === "disconnect") {
      if (!exit(record, userId)) throw notFound("Not found");

      return;
    }

    setMuted(record, userId, action === "mute");
  };

  const setMuted = (record: RoomRecord, userId: number, muted: boolean): void => {
    const key = `${record.room.id}:${userId}`;
    const state = calls();

    if (muted) state.muted.add(key);
    else state.muted.delete(key);

    const events: Outgoing[] = [];

    endStreamIfOrphaned(record, events);
    events.unshift(presenceEvent(record));

    if (record.room.kind === "stage") events.push(stageEvent(record));

    events.push(...roleChanged(record, userId));
    ctx.publish(events);
  };

  const stageDetail = (roomId: number): StageDetail => {
    const record = stageRoom(roomId);

    return { stage: stageState(record), users: ctx.usersFor(record.memberIds) };
  };

  const setRole = (record: RoomRecord, userId: number, role: StageRole): void => {
    const stage = stageOf(record);
    const hosts = [...stage.roles.values()].filter((value) => value === "host").length;

    if (stage.roles.get(userId) === "host" && role !== "host" && hosts <= 1) {
      throw validation("role", "A stage needs at least one host");
    }

    stage.roles.set(userId, role);

    if (role !== "listener") stage.hands.delete(userId);

    const events: Outgoing[] = [];

    endStreamIfOrphaned(record, events);
    events.unshift(stageEvent(record));
    events.push(presenceEvent(record), ...roleChanged(record, userId));
    ctx.publish(events);
  };

  const changeRole = (roomId: number, targetId: number, role: StageRole): StageState => {
    const record = stageRoom(roomId);

    if (!canManageStage(record)) {
      throw forbidden("Only hosts and administrators can change stage roles");
    }

    const userId = userForMembership(record, targetId);

    if (userId === null) throw notFound("Not found");

    setRole(record, userId, role);

    return stageState(record);
  };

  const setHand = (record: RoomRecord, userId: number, raised: boolean): void => {
    const stage = stageOf(record);

    if (raised) stage.hands.set(userId, new Date(ctx.now()).toISOString());
    else stage.hands.delete(userId);

    ctx.publish([stageEvent(record)]);
  };

  const raiseHand = (roomId: number): StageState => {
    const record = stageRoom(roomId);

    if (roleOf(record, VIEWER_ID) !== "listener") {
      throw validation("role", "Only listeners can raise a hand");
    }

    setHand(record, VIEWER_ID, true);

    return stageState(record);
  };

  const lowerHand = (roomId: number, targetId: string | null): StageState => {
    const record = stageRoom(roomId);
    let userId = VIEWER_ID;

    if (targetId !== null && targetId.trim() !== "") {
      if (!canManageStage(record)) {
        throw forbidden("Only hosts and administrators can change stage roles");
      }

      const found = userForMembership(record, Number(targetId));

      if (found === null) throw notFound("Not found");

      userId = found;
    }

    setHand(record, userId, false);

    return stageState(record);
  };

  const goLive = (record: RoomRecord, userId: number, quality: StreamQuality): StageStream => {
    const stage = stageOf(record);
    const roomId = record.room.id;

    if (roleOf(record, userId) === "listener") {
      throw forbidden("Only hosts and speakers can go live");
    }

    if (calls().muted.has(`${roomId}:${userId}`)) throw forbidden("You are muted on this stage");

    const entry = calls().calls.get(roomId)?.get(userId);

    if (entry === undefined) throw forbidden("Join the stage before going live");

    if (stage.live !== null && stage.live.userId !== userId) {
      throw conflict(`${nameOf(stage.live.userId)} is already live`);
    }

    const live: StageStream = {
      id: calls().nextStreamId++,
      membershipId: membershipId(record, userId),
      userId,
      identity: entry.identities.at(-1) ?? null,
      quality,
      startedAt: new Date(ctx.now()).toISOString(),
    };

    stage.live = live;
    ctx.publish([stageEvent(record), presenceEvent(record)]);

    return live;
  };

  const stopStream = (roomId: number, streamId: string | null): void => {
    const record = stageRoom(roomId);
    const stage = stageOf(record);
    const live = stage.live;

    if (live === null) return;

    if (streamId !== null && streamId.trim() !== "" && Number(streamId) !== live.id) return;

    if (live.userId !== VIEWER_ID && !canManageStage(record)) {
      throw forbidden("Only the presenter, hosts and administrators can end a stream");
    }

    endStream(record, live.userId !== VIEWER_ID);
  };

  /** Ends the live stream; a host ending someone else's tells the presenter's tab. */
  const endStream = (record: RoomRecord, byHost: boolean): void => {
    const stage = stageOf(record);
    const live = stage.live;

    if (live === null) return;

    stage.live = null;

    const events: Outgoing[] = [stageEvent(record), presenceEvent(record)];

    if (byHost && live.userId === VIEWER_ID) {
      events.push({
        topic: "user",
        type: "stage.stream.stopped",
        data: { roomId: record.room.id },
      });
    }

    ctx.publish(events);
  };

  const role = (value: string | null): StageRole => {
    const found = STAGE_ROLES.find((candidate) => candidate === value);

    if (found === undefined) throw validation("role", "Unknown stage role");

    return found;
  };

  const quality = (value: string | null): StreamQuality => {
    const found = QUALITIES.find((candidate) => candidate === value);

    if (found === undefined) throw validation("quality", "Unknown stream quality");

    return found;
  };

  const moderation = (value: string | null): HuddleModeration => {
    const found = MODERATIONS.find((candidate) => candidate === value);

    if (found === undefined) throw validation("action", "Unknown moderation action");

    return found;
  };

  const ok = (json: HuddlePresenceList | HuddleDetail | StageDetail | StageState) => ({
    status: 200,
    json,
  });

  const routes: Route[] = [
    route("GET", /^\/huddles$/, () => ok(list())),
    route("GET", /^\/rooms\/(\d+)\/huddle$/, (request) => ok(detail(firstId(request)))),
    route("POST", /^\/rooms\/(\d+)\/huddle$/, (request) => ({
      status: 201,
      json: join(firstId(request)),
    })),
    route("POST", /^\/rooms\/(\d+)\/huddle\/leave$/, (request) => {
      leave(firstId(request));

      return noContent();
    }),
    route("POST", /^\/rooms\/(\d+)\/huddle\/moderation$/, (request) => {
      const target = intField(request.body, "membershipId");

      if (target === null) throw validation("membershipId", "membershipId is required");

      moderate(firstId(request), target, moderation(stringField(request.body, "action")));

      return noContent();
    }),
    route("GET", /^\/rooms\/(\d+)\/stage$/, (request) => ok(stageDetail(firstId(request)))),
    route("PATCH", /^\/rooms\/(\d+)\/stage\/members\/(\d+)$/, (request) =>
      ok(
        changeRole(firstId(request), request.ids[1] ?? 0, role(stringField(request.body, "role"))),
      ),
    ),
    route("POST", /^\/rooms\/(\d+)\/stage\/hand$/, (request) => ok(raiseHand(firstId(request)))),
    route("DELETE", /^\/rooms\/(\d+)\/stage\/hand$/, (request) =>
      ok(lowerHand(firstId(request), request.query.get("membershipId"))),
    ),
    route("POST", /^\/rooms\/(\d+)\/stage\/stream$/, (request) => ({
      status: 201,
      json: goLive(
        stageRoom(firstId(request)),
        VIEWER_ID,
        quality(stringField(request.body, "quality")),
      ),
    })),
    route("DELETE", /^\/rooms\/(\d+)\/stage\/stream$/, (request) => {
      stopStream(firstId(request), request.query.get("streamId"));

      return noContent();
    }),
  ];

  const ring = (record: RoomRecord, callerId: number, event: HuddleRingEvent): void => {
    const state: HuddleRingState = event === "ended" ? "read" : "unread";

    ctx.publish([
      {
        topic: "user",
        type: "huddle.ring",
        data: {
          activityItemId: null,
          event,
          state,
          roomId: record.room.id,
          roomName: ctx.displayName(record),
          callerName: nameOf(callerId),
          silent: false,
        },
      },
    ]);
  };

  const done: MockResponse = { status: 200, json: { ok: true } };

  return {
    routes,
    control(action, input) {
      switch (action) {
        case "huddle-join": {
          const record = callRoom(input.int("roomId"));
          const userId = input.int("userId");

          enter(record, userId, identity(), false);

          return done;
        }

        case "huddle-leave":
          exit(callRoom(input.int("roomId")), input.int("userId"));

          return done;
        case "huddle-mute": {
          // A host (someone else) muting or unmuting anyone, the viewer included.
          setMuted(callRoom(input.int("roomId")), input.int("userId"), input.flag("muted", true));

          return done;
        }

        case "stage-role": {
          const record = stageRoom(input.int("roomId"));

          setRole(record, input.int("userId"), role(input.text("role")));

          return done;
        }

        case "stage-hand":
          setHand(stageRoom(input.int("roomId")), input.int("userId"), input.flag("raised", true));

          return done;
        case "stage-live": {
          const record = stageRoom(input.int("roomId"));

          if (input.flag("on", true)) {
            goLive(record, input.int("userId"), quality(input.optionalText("quality") ?? "720p15"));
          } else {
            endStream(record, true);
          }

          return done;
        }

        case "huddle-ring": {
          const record = callRoom(input.int("roomId"));
          const event = input.optionalText("event") ?? "started";

          if (event !== "started" && event !== "missed" && event !== "ended") {
            throw validation("event", "event must be started, missed or ended");
          }

          ring(record, input.int("userId"), event);

          return done;
        }

        default:
          return null;
      }
    },
  };
}
