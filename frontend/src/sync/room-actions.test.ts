import { afterEach, describe, expect, it } from "@effect/vitest";
import { Effect, Predicate } from "effect";
import { NotFound, Validation } from "../api/errors.ts";
import {
  FakeApi,
  roomDetailFixture,
  sidebarFixture,
  sidebarRowFixture,
  userFixture,
} from "../api/testing.ts";
import type { RoomForm } from "../gen/RoomForm.ts";
import { beginRoomRequest } from "../store/join-state.ts";
import { mutations, store } from "../store/store.ts";
import * as rooms from "./room-actions.ts";
import { invalidateRoom } from "./room-refresh.ts";

const form: RoomForm = {
  type: "closed",
  roomId: null,
  name: "New room",
  iconName: null,
  displayName: "New room",
  userIds: [7],
  memberIds: [],
  candidateIds: [7, 8],
  displayMemberIds: [],
  users: [userFixture(8, "Grace Hopper")],
  allowedTypes: ["closed", "direct"],
  conversionTypes: [],
  canSubmit: true,
  canDelete: false,
  canLeave: false,
  groupCapable: false,
  defaultInvolvement: "everything",
  stageRoles: [],
};

const seed = () => {
  mutations.reset();
  mutations.loadSidebar(sidebarFixture([sidebarRowFixture(20, "Room")]));
  mutations.setRoomDetail(roomDetailFixture(20), beginRoomRequest());
};

describe("room management actions", () => {
  afterEach(() => mutations.reset());

  it.effect("loads form facts and profiles using the selected type query", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;

      yield* fake.reply("GET /rooms/new", form);
      yield* fake.reply("GET /rooms/20/edit", { ...form, roomId: 20 });

      expect(yield* rooms.newForm("closed")).toEqual(form);
      expect((yield* rooms.editForm(20)).roomId).toBe(20);
      expect((yield* fake.requests)[0]?.query).toEqual({ type: "closed" });
      expect(store.getState().users[8]?.name).toBe("Grace Hopper");
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("lands create/update replies and sends patch nulls without adding omitted fields", () =>
    Effect.gen(function* () {
      seed();
      const fake = yield* FakeApi;
      const detail = { ...roomDetailFixture(30), memberCount: 6, memberPreviewIds: [7, 8] };
      const row = sidebarRowFixture(30, "New board", "board");

      const result = {
        room: row.room,
        detail: { ...detail, room: row.room, displayName: row.displayName },
        row,
      };

      yield* fake.reply("POST /rooms", result);
      yield* fake.reply("PATCH /rooms/30", result);
      yield* rooms.create({
        type: "board",
        clientRoomId: "board-1",
        name: "New board",
        iconName: null,
        userIds: [7, 8],
      });
      yield* rooms.update(30, { type: "board", iconName: null, userIds: [7, 8] });

      expect(store.getState().sidebar.rows[30]?.room.kind).toBe("board");
      expect(store.getState().rooms[30]?.detail?.memberCount).toBe(6);
      expect((yield* fake.requests).at(-1)?.body).toEqual({
        type: "board",
        iconName: null,
        userIds: [7, 8],
      });
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("accepts self-removal success and preserves readable hidden membership", () =>
    Effect.gen(function* () {
      seed();
      const fake = yield* FakeApi;
      const detail = roomDetailFixture(20);

      yield* fake.reply("PATCH /rooms/20", { room: detail.room, detail, row: null });
      yield* rooms.update(20, { type: "open" });
      expect(store.getState().sidebar.rows[20]).toBeUndefined();
      expect(store.getState().rooms[20]?.detail).toEqual(detail);

      yield* fake.reply("PATCH /rooms/20", { room: detail.room, detail: null, row: null });
      const result = yield* rooms.update(20, { type: "closed", userIds: [8] });

      expect(result.detail).toBeNull();
      expect(store.getState().rooms[20]?.detail).toBeNull();
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("invalidates locally confirmed delete/leave without relying on a socket", () =>
    Effect.gen(function* () {
      seed();
      const fake = yield* FakeApi;

      yield* fake.reply("DELETE /rooms/20", { roomId: 20, deleted: true });
      yield* rooms.remove(20);
      expect(store.getState().rooms[20]?.detail).toBeNull();
      expect(store.getState().sidebar.rows[20]).toBeUndefined();

      seed();
      yield* fake.reply("DELETE /rooms/20/membership", { roomId: 20, deleted: false });
      expect((yield* rooms.leaveDirect(20)).deleted).toBe(false);
      expect(store.getState().rooms[20]?.detail).toBeNull();
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect(
    "doesn't bring back a room the viewer was removed from while a save was on its way",
    () =>
      Effect.gen(function* () {
        seed();
        const fake = yield* FakeApi;
        const detail = roomDetailFixture(20);
        const row = sidebarRowFixture(20, "Renamed");

        // Another admin removes the viewer while the save is pending: sync drops the row and the
        // room (as the engine does), then the older reply arrives.
        yield* fake.route("PATCH /rooms/20", () =>
          Effect.sync(() => {
            mutations.applyEvents(
              [
                {
                  seq: 1,
                  topic: "user:7",
                  type: "sidebar.row.removed",
                  data: { roomId: 20, refreshRoom: true },
                },
              ],
              0,
            );
            invalidateRoom(20);
            mutations.setRoomUnavailable(20);

            return { room: row.room, detail: { ...detail, room: row.room }, row };
          }),
        );
        yield* fake.route("GET /rooms/20", () =>
          Effect.fail(new NotFound({ message: "Not found" })),
        );

        const result = yield* rooms.update(20, { type: "open", name: "Renamed" });

        expect(result.row?.displayName).toBe("Renamed");
        expect(store.getState().sidebar.rows[20]).toBeUndefined();
        expect(store.getState().rooms[20]?.detail).toBeNull();
        expect(
          (yield* fake.requests).map((request) => `${request.method} ${request.path}`),
        ).toEqual(["PATCH /rooms/20", "GET /rooms/20"]);
      }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("reads a room again instead of landing a reply older than a sync change", () =>
    Effect.gen(function* () {
      seed();
      const fake = yield* FakeApi;
      const stale = sidebarRowFixture(20, "Mine");
      const newer = sidebarRowFixture(20, "Theirs");

      yield* fake.route("PATCH /rooms/20", () =>
        Effect.sync(() => {
          mutations.applyEvents(
            [{ seq: 1, topic: "user:7", type: "sidebar.row.upserted", data: newer }],
            0,
          );
          invalidateRoom(20);

          return {
            room: stale.room,
            detail: { ...roomDetailFixture(20), displayName: "Mine" },
            row: stale,
          };
        }),
      );
      yield* fake.reply("GET /rooms/20", { ...roomDetailFixture(20), displayName: "Fetched" });

      yield* rooms.update(20, { type: "open", name: "Mine" });

      expect(store.getState().sidebar.rows[20]?.displayName).toBe("Theirs");
      // The re-read detail carries the newer row's facts.
      expect(store.getState().rooms[20]?.detail?.displayName).toBe("Theirs");
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("retains loaded state and structured validation fields after refusal", () =>
    Effect.gen(function* () {
      seed();
      const fake = yield* FakeApi;
      const before = store.getState();

      yield* fake.route("PATCH /rooms/20", () =>
        Effect.fail(
          new Validation({
            message: "Invalid icon",
            fields: { iconName: ["is not a known icon"] },
          }),
        ),
      );

      const failure = yield* rooms
        .update(20, { type: "open", iconName: "unknown" })
        .pipe(Effect.flip);

      expect(Predicate.isTagged(failure, "Validation") ? failure.fields : null).toEqual({
        iconName: ["is not a known icon"],
      });
      expect(store.getState()).toBe(before);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});
