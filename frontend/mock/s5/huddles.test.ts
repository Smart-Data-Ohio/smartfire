import { describe, expect, it } from "vitest";
import type { HuddleCredentials } from "../../src/gen/HuddleCredentials.ts";
import type { HuddleDetail } from "../../src/gen/HuddleDetail.ts";
import type { HuddlePresenceList } from "../../src/gen/HuddlePresenceList.ts";
import type { StageDetail } from "../../src/gen/StageDetail.ts";
import type { StageState } from "../../src/gen/StageState.ts";
import type { StageStream } from "../../src/gen/StageStream.ts";
import type { Json } from "../json.ts";
import { collect, errorOf, expectStatus, get, harness, send } from "../s2/testing.ts";
import { SEED_IDS } from "../server.ts";
import { MOCK_LIVEKIT_URL } from "./huddles.ts";

const { rooms, users, viewer } = SEED_IDS;

const lounge = rooms.lounge;

const townHall = rooms.townHall;

describe("huddles", () => {
  it("seeds a call in the Lounge", async () => {
    const { server } = harness();
    const list = await get<HuddlePresenceList>(server, "/api/v1/huddles");

    expect(list.rooms).toHaveLength(1);
    expect(list.rooms[0]?.roomId).toBe(lounge);
    expect(list.rooms[0]?.participants.map((entry) => entry.userId)).toEqual([
      users.jonah,
      users.maya,
    ]);
    expect(list.users.map((user) => user.id).sort()).toEqual([users.maya, users.jonah].sort());
  });

  it("joins with a fake token and leaves", async () => {
    const { server } = harness();
    const events = collect(server);

    const credentials = await expectStatus<HuddleCredentials>(
      server,
      "POST",
      `/api/v1/rooms/${lounge}/huddle`,
      {},
      201,
    );

    expect(credentials.url).toBe(MOCK_LIVEKIT_URL);
    expect(credentials.canPublish).toBe(true);
    expect(JSON.parse(credentials.token)).toMatchObject({
      identity: credentials.identity,
      userId: viewer,
      canPublish: true,
    });

    const detail = await get<HuddleDetail>(server, `/api/v1/rooms/${lounge}/huddle`);
    const me = detail.presence.participants.find((entry) => entry.userId === viewer);

    expect(me?.identities).toEqual([credentials.identity]);
    expect(events.some((event) => event.type === "huddle.presence")).toBe(true);

    await expectStatus(server, "POST", `/api/v1/rooms/${lounge}/huddle/leave`, {}, 204);

    const after = await get<HuddleDetail>(server, `/api/v1/rooms/${lounge}/huddle`);

    expect(after.presence.participants.map((entry) => entry.userId)).not.toContain(viewer);
  });

  it("moves the viewer out of another call on join", async () => {
    const { server } = harness();

    await send(server, "POST", `/api/v1/rooms/${lounge}/huddle`, {});
    await send(server, "POST", `/api/v1/rooms/${rooms.general}/huddle`, {});

    const loungeCall = await get<HuddleDetail>(server, `/api/v1/rooms/${lounge}/huddle`);

    expect(loungeCall.presence.participants.map((entry) => entry.userId)).not.toContain(viewer);
  });

  it("lets the administrator mute and disconnect, never themselves", async () => {
    const { server } = harness();
    const detail = await get<HuddleDetail>(server, `/api/v1/rooms/${lounge}/huddle`);
    const maya = detail.presence.participants.find((entry) => entry.userId === users.maya);
    const path = `/api/v1/rooms/${lounge}/huddle/moderation`;

    await expectStatus(
      server,
      "POST",
      path,
      { membershipId: maya?.membershipId ?? 0, action: "mute" },
      204,
    );

    const muted = await get<HuddleDetail>(server, `/api/v1/rooms/${lounge}/huddle`);

    expect(
      muted.presence.participants.find((entry) => entry.userId === users.maya)?.serverMuted,
    ).toBe(true);

    await expectStatus(
      server,
      "POST",
      path,
      { membershipId: maya?.membershipId ?? 0, action: "disconnect" },
      204,
    );

    const left = await get<HuddleDetail>(server, `/api/v1/rooms/${lounge}/huddle`);

    expect(left.presence.participants.map((entry) => entry.userId)).toEqual([users.jonah]);

    const own = await expectStatus<Json>(
      server,
      "POST",
      path,
      { membershipId: 100 + lounge, action: "mute" },
      422,
    );

    expect(errorOf(own).tag).toBe("Validation");

    const elsewhere = await send(
      server,
      "POST",
      `/api/v1/rooms/${rooms.general}/huddle/moderation`,
      {
        membershipId: 1,
        action: "mute",
      },
    );

    expect(elsewhere.status).toBe(404);
  });

  it("tells the viewer when a host mutes them, and their token stops publishing", async () => {
    const { server } = harness();
    const events = collect(server);

    await send(server, "POST", `/api/v1/rooms/${lounge}/huddle`, {});
    await server.handle({
      method: "POST",
      path: "/__mock/huddle-mute",
      body: { roomId: lounge, userId: viewer, muted: true },
    });

    const role = events.find((event) => event.type === "huddle.role");

    expect(role?.data).toEqual({ roomId: lounge, stageRole: null, serverMuted: true });

    const again = await expectStatus<HuddleCredentials>(
      server,
      "POST",
      `/api/v1/rooms/${lounge}/huddle`,
      {},
      201,
    );

    expect(again.canPublish).toBe(false);
  });

  it("publishes join notices for other people", async () => {
    const { server } = harness();
    const events = collect(server);

    await server.handle({
      method: "POST",
      path: "/__mock/huddle-join",
      body: { roomId: lounge, userId: users.priya },
    });

    const notice = events.find((event) => event.type === "huddle.notice");

    expect(notice?.data).toMatchObject({ kind: "joined", userId: users.priya, inCall: false });
  });
});

describe("stages", () => {
  it("serves the roster with the viewer as host", async () => {
    const { server } = harness();
    const detail = await get<StageDetail>(server, `/api/v1/rooms/${townHall}/stage`);
    const me = detail.stage.members.find((member) => member.userId === viewer);

    expect(me?.role).toBe("host");
    expect(detail.stage.members[0]?.role).toBe("host");
    expect(detail.stage.live).toBeNull();
  });

  it("changes roles, queues hands and refuses the last host stepping down", async () => {
    const { server } = harness();
    const events = collect(server, [`room:${townHall}`]);
    const detail = await get<StageDetail>(server, `/api/v1/rooms/${townHall}/stage`);
    const grace = detail.stage.members.find((member) => member.userId === users.grace);

    await server.handle({
      method: "POST",
      path: "/__mock/stage-hand",
      body: { roomId: townHall, userId: users.grace },
    });

    const raised = events.findLast((event) => event.type === "stage.updated");

    expect(
      raised?.type === "stage.updated" &&
        raised.data.members.find((m) => m.userId === users.grace)?.handRaisedAt,
    ).toBeTruthy();

    const promoted = await expectStatus<StageState>(
      server,
      "PATCH",
      `/api/v1/rooms/${townHall}/stage/members/${grace?.membershipId ?? 0}`,
      { role: "speaker" },
      200,
    );

    const after = promoted.members.find((member) => member.userId === users.grace);

    expect(after?.role).toBe("speaker");
    expect(after?.handRaisedAt).toBeNull();

    const own = await send(
      server,
      "PATCH",
      `/api/v1/rooms/${townHall}/stage/members/${100 + townHall}`,
      { role: "listener" },
    );

    expect(own.status).toBe(422);
  });

  it("goes live only from the call, and a host ending it tells the presenter", async () => {
    const { server } = harness();
    const events = collect(server, [`room:${townHall}`]);
    const path = `/api/v1/rooms/${townHall}/stage/stream`;

    const outside = await send(server, "POST", path, { quality: "720p15" });

    expect(outside.status).toBe(403);

    await send(server, "POST", `/api/v1/rooms/${townHall}/huddle`, {});

    const stream = await expectStatus<StageStream>(
      server,
      "POST",
      path,
      { quality: "1080p30" },
      201,
    );

    expect(stream.userId).toBe(viewer);
    expect(stream.quality).toBe("1080p30");

    await server.handle({
      method: "POST",
      path: "/__mock/stage-live",
      body: { roomId: townHall, on: false },
    });

    expect(events.some((event) => event.type === "stage.stream.stopped")).toBe(true);
  });
});
