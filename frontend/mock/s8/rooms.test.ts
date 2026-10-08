import { describe, expect, it } from "vitest";
import type { CreateRoom } from "../../src/gen/CreateRoom.ts";
import type { RoomForm } from "../../src/gen/RoomForm.ts";
import type { RoomMutation } from "../../src/gen/RoomMutation.ts";
import { notFound } from "../http.ts";
import { field } from "../json.ts";
import { dispatch, type S2Context } from "../s2/context.ts";
import { collect, expectStatus, get, harness, send } from "../s2/testing.ts";
import { buildWorld } from "../s3/seed.ts";
import { SEED_IDS } from "../server.ts";
import { createRoomManagement } from "./rooms.ts";

const { rooms, users, viewer } = SEED_IDS;

describe("room management mock", () => {
  it("replays each room creation without allocating ids, granting members or publishing again", async () => {
    const { server } = harness();
    const events = collect(server);

    for (const type of ["open", "closed", "voice", "stage", "board"] as const) {
      const body: CreateRoom =
        type === "open"
          ? { type, clientRoomId: `replay-${type}`, name: " Once ", iconName: " :FIRE: " }
          : {
              type,
              clientRoomId: `replay-${type}`,
              name: " Once ",
              iconName: " :FIRE: ",
              userIds: [users.maya, users.dana],
            };

      const first = await expectStatus<RoomMutation>(server, "POST", "/api/v1/rooms", body, 201);
      const count = events.length;
      const retry = { ...body, clientRoomId: ` ${body.clientRoomId} `, iconName: "fire" };

      if ("userIds" in retry)
        retry.userIds = [users.dana, users.maya, users.maya, ...(type === "stage" ? [viewer] : [])];

      const replay = await expectStatus<RoomMutation>(server, "POST", "/api/v1/rooms", retry, 200);

      expect(replay).toEqual(first);
      expect(events).toHaveLength(count);

      const next = await expectStatus<RoomMutation>(
        server,
        "POST",
        "/api/v1/rooms",
        { ...body, clientRoomId: `next-${type}` },
        201,
      );

      expect(next.room.id).toBe(first.room.id + 1);

      if (next.detail !== null && first.detail !== null)
        expect(next.detail.membership.id).toBe(first.detail.membership.id + 1);
    }
  });

  it("refuses changed creation parameters without allocating ids, granting members or publishing", async () => {
    const { server } = harness();
    const events = collect(server);

    for (const type of ["open", "closed", "voice", "stage", "board"] as const) {
      const common = { clientRoomId: `conflict-${type}`, name: "Once", iconName: "fire" };

      const body =
        type === "open" ? { ...common, type } : { ...common, type, userIds: [viewer, users.maya] };

      const first = await expectStatus<RoomMutation>(server, "POST", "/api/v1/rooms", body, 201);

      const count = events.length;

      const changedType =
        type === "open"
          ? { ...common, type: "closed", userIds: [viewer, users.maya] }
          : { ...common, type: "open" };

      const retries = [
        { ...body, name: "Different" },
        { ...body, iconName: "smile" },
        { ...body, iconName: "invalid_retry_icon" },
        changedType,
        ...(type === "open" ? [] : [{ ...body, userIds: [viewer, users.sam] }]),
      ];

      for (const retry of retries) {
        const response = await send(server, "POST", "/api/v1/rooms", retry);

        expect(response.status).toBe(409);
        expect(field(field(response.json, "error"), "_tag")).toBe("Conflict");
        expect(field(field(response.json, "error"), "message")).toBe(
          "clientRoomId was already used with different room parameters",
        );
        expect(events).toHaveLength(count);
      }

      expect(await expectStatus<RoomMutation>(server, "POST", "/api/v1/rooms", body, 200)).toEqual(
        first,
      );

      const next = await expectStatus<RoomMutation>(
        server,
        "POST",
        "/api/v1/rooms",
        { ...body, clientRoomId: `next-conflict-${type}` },
        201,
      );

      expect(next.room.id).toBe(first.room.id + 1);

      if (next.detail !== null && first.detail !== null)
        expect(next.detail.membership.id).toBe(first.detail.membership.id + 1);
    }
  });

  it("rejects missing and blank creation keys in the structured validation shape", async () => {
    const { server } = harness();
    const events = collect(server);

    for (const body of [
      { type: "open", name: "Never" },
      { type: "open", name: "Never", clientRoomId: "" },
      { type: "open", name: "Never", clientRoomId: " \t\n " },
    ]) {
      const response = await send(server, "POST", "/api/v1/rooms", body);

      expect(response.status).toBe(422);
      expect(field(field(response.json, "error"), "_tag")).toBe("Validation");
      expect(field(response.json, "error")).toMatchObject({
        fields: { clientRoomId: ["can't be blank"] },
      });
    }

    expect(events).toEqual([]);
  });

  it("supplies defaults for every type and readable form capabilities", async () => {
    const { server } = harness();

    for (const type of ["open", "closed", "direct", "voice", "stage", "board"] as const) {
      const form = await get<RoomForm>(server, `/api/v1/rooms/new?type=${type}`);

      expect(form.type).toBe(type);
      expect(form.roomId).toBeNull();

      if (type === "open") expect(form.userIds).toContain(viewer);
      else expect(form.userIds).toEqual(type === "direct" ? [] : [viewer]);
      expect(form.canSubmit).toBe(true);
      expect(form.canDelete).toBe(false);
      expect(form.candidateIds).not.toContain(users.dana);
    }

    const edit = await get<RoomForm>(server, `/api/v1/rooms/${rooms.dmMaya}/edit`);

    expect(edit).toMatchObject({
      type: "direct",
      canSubmit: false,
      canLeave: true,
      canDelete: true,
    });
    expect(edit.displayMemberIds).toEqual([users.maya]);
  });

  it("creates every non-direct kind, preserving explicit membership and stage creator host", async () => {
    const { server } = harness();
    const events = collect(server);

    for (const type of ["open", "closed", "voice", "stage", "board"] as const) {
      const body =
        type === "open"
          ? { type, clientRoomId: `room-${type}`, name: null, iconName: " :FIRE: " }
          : {
              type,
              clientRoomId: `room-${type}`,
              name: null,
              iconName: " :FIRE: ",
              userIds: [users.maya, users.dana],
            };

      const created = await expectStatus<RoomMutation>(server, "POST", "/api/v1/rooms", body, 201);

      expect(created.room).toMatchObject({ kind: type, name: null, iconName: "fire" });

      if (type === "open" || type === "stage") {
        expect(created.detail).not.toBeNull();
      } else {
        expect(created.detail).toBeNull();
        expect(created.row).toBeNull();
        expect(
          (await server.handle({ method: "GET", path: `/api/v1/rooms/${created.room.id}` })).status,
        ).toBe(404);
      }

      if (type === "stage") expect(created.detail?.membership.stageRole).toBe("host");
    }

    expect(
      events.every(
        (event) => event.type === "sidebar.row.upserted" || event.type === "sidebar.row.removed",
      ),
    ).toBe(true);
    expect(
      events.every((event) => "refreshRoom" in event.data && event.data.refreshRoom === true),
    ).toBe(true);
  });

  it("distinguishes preserve/clear patches, converts only open/closed, and returns self-removal success", async () => {
    const { server } = harness();
    const path = `/api/v1/rooms/${rooms.quiet}`;

    const renamed = await expectStatus<RoomMutation>(
      server,
      "PATCH",
      path,
      { type: "open", name: "Renamed", iconName: "fire" },
      200,
    );

    const retained = await expectStatus<RoomMutation>(server, "PATCH", path, { type: "open" }, 200);

    expect(retained.room.name).toBe(renamed.room.name);
    expect(retained.room.iconName).toBe("fire");
    expect(retained.detail?.memberCount).toBe(2);

    const cleared = await expectStatus<RoomMutation>(
      server,
      "PATCH",
      path,
      { type: "open", name: null, iconName: null },
      200,
    );

    expect(cleared.room.name).toBeNull();
    expect(cleared.room.iconName).toBeNull();
    const conversion = await send(server, "PATCH", path, { type: "board", userIds: [viewer] });

    expect(conversion.status).toBe(422);
    expect(field(field(conversion.json, "error"), "_tag")).toBe("Validation");
    expect(field(field(conversion.json, "error"), "message")).toBe(
      "Type cannot convert this room to that type",
    );
    expect(field(field(conversion.json, "error"), "fields")).toEqual({
      type: ["cannot convert this room to that type"],
    });

    const spacedIcon = await expectStatus<RoomMutation>(
      server,
      "PATCH",
      path,
      { type: "open", iconName: ": FIRE :" },
      200,
    );

    expect(spacedIcon.room.iconName).toBe("fire");

    const converted = await expectStatus<RoomMutation>(
      server,
      "PATCH",
      path,
      { type: "closed", userIds: [viewer, users.sam] },
      200,
    );

    const openAgain = await expectStatus<RoomMutation>(
      server,
      "PATCH",
      path,
      { type: "open" },
      200,
    );

    expect(converted.detail?.memberCount).toBe(2);
    expect(openAgain.detail?.memberCount).toBeGreaterThan(2);

    const removed = await expectStatus<RoomMutation>(
      server,
      "PATCH",
      path,
      { type: "closed", userIds: [users.maya] },
      200,
    );

    expect(removed.room.kind).toBe("closed");
    expect(removed.detail).toBeNull();
    expect((await server.handle({ method: "GET", path })).status).toBe(404);
  });

  it("initializes new stage members as listeners and preserves mentions defaults", async () => {
    const { server } = harness();

    const created = await expectStatus<RoomMutation>(
      server,
      "POST",
      "/api/v1/rooms",
      {
        type: "stage",
        clientRoomId: "new-stage",
        name: "Stage",
        iconName: null,
        userIds: [viewer, users.priya],
      },
      201,
    );

    const edit = await get<RoomForm>(server, `/api/v1/rooms/${created.room.id}/edit`);

    expect(created.detail?.membership.involvement).toBe("mentions");
    expect(edit.defaultInvolvement).toBe("mentions");
    expect(edit.stageRoles).toEqual([
      { userId: viewer, role: "host" },
      { userId: users.priya, role: "listener" },
    ]);
  });

  it("refuses invalid icon and last stage host before changing fields or publications", async () => {
    const { server } = harness();
    const path = `/api/v1/rooms/${rooms.townHall}`;
    const before = await get<RoomForm>(server, `${path}/edit`);
    const events = collect(server);

    const invalid = await send(server, "PATCH", path, {
      type: "stage",
      name: "Changed",
      iconName: "missing_icon",
      userIds: [viewer],
    });

    expect(invalid.status).toBe(422);
    expect(field(field(invalid.json, "error"), "fields")).toEqual({
      iconName: ["is not a known icon"],
    });
    expect(field(field(invalid.json, "error"), "message")).toBe("Icon name is not a known icon");
    expect(
      (await send(server, "PATCH", path, { type: "stage", name: "Changed", userIds: [users.maya] }))
        .status,
    ).toBe(422);

    expect(await get<RoomForm>(server, `${path}/edit`)).toEqual(before);
    expect(events).toEqual([]);
    const unknown = await send(server, "PATCH", path, { type: "stage", userIds: [999999] });

    expect(unknown.status).toBe(422);
    expect(field(field(unknown.json, "error"), "fields")).toEqual({
      userIds: ["Promote another host before removing Riel St. Amand"],
    });
    expect(field(field(unknown.json, "error"), "message")).toBe(
      "Promote another host before removing Riel St. Amand",
    );
    expect(await get<RoomForm>(server, `${path}/edit`)).toEqual(before);
    expect(events).toEqual([]);

    const empty = await expectStatus<RoomMutation>(
      server,
      "PATCH",
      path,
      { type: "stage", userIds: [] },
      200,
    );

    expect(empty.detail).toBeNull();
    expect(empty.row).toBeNull();
  });

  it("keeps ordinary members read-only and respects pair/group direct deletion policy", async () => {
    const { clock } = harness();
    const world = buildWorld(clock.now(), 1);
    const user = world.users.get(viewer);

    if (user === undefined) throw new Error("Missing seeded viewer");
    world.users.set(viewer, { ...user, role: "member" });

    const unsupported = () => {
      throw new Error("Unused context seam");
    };

    const ctx: S2Context = {
      world: () => world,
      now: () => clock.now(),
      scheduler: clock,
      publish: () => {},
      uuid: () => "",
      hex: () => "",
      mentionables: () => [],
      roomOr404: (id) => {
        const room = world.rooms.get(id);

        if (room === undefined) throw notFound();

        return room;
      },
      usersFor: (ids) => [...world.users.values()].filter((user) => [...ids].includes(user.id)),
      displayName: (record) => record.room.name ?? "",
      sidebarRow: unsupported,
      roomDetail: unsupported,
      postToRoom: unsupported,
    };

    const management = createRoomManagement(
      ctx,
      { roomCreationRestricted: () => false, hasIcon: () => false },
      {
        roomRoles: () => new Map(),
        reviseRoom: () => {},
        removeRoom: () => {},
      },
    );

    const request = async (method: string, path: string, body?: Parameters<typeof dispatch>[4]) => {
      const run = dispatch(management.routes, method, path, new URLSearchParams(), body);

      if (run === null) throw new Error("Missing management route");

      return await run();
    };

    const edit = (await request("GET", `/rooms/${rooms.quiet}/edit`)).json;

    expect(field(edit, "canSubmit")).toBe(false);
    await expect(
      request("PATCH", `/rooms/${rooms.quiet}`, { type: "open", name: "Changed" }),
    ).rejects.toMatchObject({ status: 403 });
    await expect(request("DELETE", `/rooms/${rooms.groupDm}`)).rejects.toMatchObject({
      status: 403,
    });
    expect((await request("DELETE", `/rooms/${rooms.dmMaya}`)).status).toBe(200);
    await expect(request("PATCH", `/rooms/${rooms.groupDm}`, null)).rejects.toMatchObject({
      status: 404,
    });
  });

  it("allows direct-only leave, deleting a solo direct, and enforces CSRF", async () => {
    const { server } = harness();

    expect(
      (
        await server.handle({
          method: "POST",
          path: "/api/v1/rooms",
          body: { type: "open", name: null, iconName: null },
        })
      ).status,
    ).toBe(422);
    expect((await send(server, "DELETE", `/api/v1/rooms/${rooms.quiet}/membership`)).status).toBe(
      404,
    );
    const left = await send(server, "DELETE", `/api/v1/rooms/${rooms.groupDm}/membership`);

    expect(left.json).toEqual({ roomId: rooms.groupDm, deleted: false });
    expect(
      (await server.handle({ method: "GET", path: `/api/v1/rooms/${rooms.groupDm}/edit` })).status,
    ).toBe(404);

    const solo = await expectStatus<{ room: { id: number } }>(
      server,
      "POST",
      "/api/v1/directs",
      { userIds: [] },
      201,
    );

    expect((await send(server, "DELETE", `/api/v1/rooms/${solo.room.id}/membership`)).json).toEqual(
      { roomId: solo.room.id, deleted: true },
    );
  });
});
