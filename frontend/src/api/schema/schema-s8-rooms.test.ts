import { describe, expect, it } from "@effect/vitest";
import { Schema } from "effect";
import { createMockServer } from "../../../mock/server.ts";
import { roomDetailFixture, sidebarRowFixture, userFixture } from "../testing.ts";
import {
  CreateRoom,
  RoomForm,
  RoomLeft,
  RoomMutation,
  RoomRemoved,
  UpdateRoom,
} from "./room-management.ts";
import { SidebarRow, SidebarRowRemoved } from "./sidebar.ts";

const roundTrips = <S extends Schema.Codec<unknown, unknown>>(schema: S, wire: S["Encoded"]) =>
  expect(Schema.encodeSync(schema)(Schema.decodeUnknownSync(schema)(wire))).toEqual(wire);

describe("S8 room management schemas", () => {
  it("decodes form and successful mutation bodies from the real mock routes", async () => {
    const mock = createMockServer();

    try {
      for (const type of ["open", "closed", "direct", "voice", "stage", "board"] as const) {
        const response = await mock.handle({
          method: "GET",
          path: `/api/v1/rooms/new?type=${type}`,
        });

        expect(response.status).toBe(200);
        expect(
          Schema.encodeSync(RoomForm)(Schema.decodeUnknownSync(RoomForm)(response.json)),
        ).toEqual(response.json);
      }

      const response = await mock.handle({
        method: "POST",
        path: "/api/v1/rooms",
        headers: { "X-CSRF-Token": mock.csrfToken() },
        body: {
          type: "closed",
          clientRoomId: "mock-room",
          name: null,
          iconName: null,
          userIds: [2],
        },
      });

      expect(response.status).toBe(201);
      expect(
        Schema.encodeSync(RoomMutation)(Schema.decodeUnknownSync(RoomMutation)(response.json)),
      ).toEqual(response.json);
    } finally {
      mock.dispose();
    }
  });
  it("round-trip a readable direct form and stage role facts", () => {
    roundTrips(RoomForm, {
      type: "direct",
      roomId: 20,
      name: "Launch crew",
      iconName: null,
      displayName: "Launch crew",
      userIds: [7, 8],
      memberIds: [7, 8, 9],
      candidateIds: [10],
      displayMemberIds: [8, 9],
      users: [userFixture(8, "Grace Hopper")],
      allowedTypes: ["direct"],
      conversionTypes: [],
      canSubmit: true,
      canDelete: false,
      canLeave: true,
      groupCapable: true,
      defaultInvolvement: "everything",
      stageRoles: [{ userId: 7, role: "host" }],
    });
  });

  it("round-trip every non-direct create and patch without conflating omitted/null", () => {
    expect(() =>
      Schema.decodeUnknownSync(CreateRoom)({ type: "open", name: null, iconName: null }),
    ).toThrow();
    roundTrips(CreateRoom, { type: "open", clientRoomId: "room-key", name: null, iconName: null });
    roundTrips(UpdateRoom, { type: "open" });
    roundTrips(UpdateRoom, { type: "open", name: null, iconName: "fire" });

    for (const type of ["closed", "voice", "stage", "board"] as const) {
      roundTrips(CreateRoom, {
        type,
        clientRoomId: "room-key",
        name: "Room",
        iconName: null,
        userIds: [7],
      });
      roundTrips(UpdateRoom, { type, userIds: [] });
      roundTrips(UpdateRoom, { type, name: null, iconName: null, userIds: [7] });
    }

    for (const value of [
      { type: "direct", name: null, iconName: null, userIds: [] },
      { type: "closed", name: null, iconName: null },
      { type: "stage", userIds: [""] },
      { type: "voice", userIds: null },
    ]) {
      expect(() => Schema.decodeUnknownSync(CreateRoom)(value)).toThrow();
      expect(() => Schema.decodeUnknownSync(UpdateRoom)(value)).toThrow();
    }
  });

  it("accepts successful saves without membership and independent hidden sidebar rows", () => {
    const detail = roomDetailFixture(20);
    const row = sidebarRowFixture(20, "Room");

    roundTrips(RoomMutation, { room: detail.room, detail, row });
    roundTrips(RoomMutation, { room: detail.room, detail, row: null });
    roundTrips(RoomMutation, { room: detail.room, detail: null, row: null });
    roundTrips(RoomRemoved, { roomId: 20, deleted: true });
    roundTrips(RoomLeft, { roomId: 20, deleted: false });
  });

  it("keeps management refresh markers optional on ordinary sidebar events", () => {
    const row = sidebarRowFixture(20, "Room");

    roundTrips(SidebarRow, row);
    roundTrips(SidebarRow, { ...row, refreshRoom: true });
    roundTrips(SidebarRowRemoved, { roomId: 20 });
    roundTrips(SidebarRowRemoved, { roomId: 20, refreshRoom: true });
  });
});
