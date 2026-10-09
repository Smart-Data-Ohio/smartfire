import { afterEach, describe, expect, it } from "@effect/vitest";
import { Effect } from "effect";
import type { ApiRequest } from "../api/client.ts";
import {
  FakeApi,
  meFixture,
  roomDetailFixture,
  sidebarFixture,
  sidebarRowFixture,
  userFixture,
} from "../api/testing.ts";
import { mutations, sidebarRowClock, store } from "../store/store.ts";
import * as directs from "./direct-actions.ts";

function seed(): void {
  mutations.reset();
  mutations.setMe(meFixture);
  mutations.loadSidebar(
    sidebarFixture([
      sidebarRowFixture(1, "general"),
      sidebarRowFixture(20, "Grace and Ada", "direct", [3, 4]),
    ]),
    sidebarRowClock(),
  );
}

describe("direct actions", () => {
  afterEach(() => mutations.reset());

  it.effect("land a new DM's sidebar row at once, in name order", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;

      yield* fake.reply("POST /directs", sidebarRowFixture(30, "Ana", "direct", [5]));

      const row = yield* directs.create([5]);
      const requests = yield* fake.requests;

      expect(row.room.id).toBe(30);
      expect(requests.at(-1)?.body).toEqual({ userIds: [5] });
      expect(store.getState().sidebar.rows[30]?.displayName).toBe("Ana");
      expect(store.getState().sidebar.order).toEqual([30, 1, 20]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("mirror a rename onto the header and the sidebar row", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      let body: ApiRequest["body"];

      yield* fake.route("PATCH /directs/20", (request) => {
        body = request.body;

        return Effect.succeed({
          ...roomDetailFixture(20),
          displayName: "Launch crew",
          directMemberIds: [3, 4],
        });
      });

      yield* directs.rename(20, "  Launch crew ");

      expect(body).toEqual({ name: "Launch crew" });
      expect(store.getState().rooms[20]?.detail?.displayName).toBe("Launch crew");
      expect(store.getState().sidebar.rows[20]?.displayName).toBe("Launch crew");
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("send a blank name as null", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;

      yield* fake.reply("PATCH /directs/20", roomDetailFixture(20));
      yield* directs.rename(20, "   ");

      const requests = yield* fake.requests;

      expect(requests.at(-1)?.body).toEqual({ name: null });
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("keep the candidates' profiles in the store", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;

      yield* fake.reply("GET /directs/candidates", {
        candidates: [{ userId: 5, agent: false, starred: true }],
        users: [userFixture(5, "Ana Lima")],
      });

      const list = yield* directs.candidates();

      expect(list).toEqual([{ userId: 5, agent: false, starred: true }]);
      expect(store.getState().users[5]?.name).toBe("Ana Lima");
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("update the member list after adding people", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;

      yield* fake.reply("POST /directs/20/members", {
        ...roomDetailFixture(20),
        displayName: "Grace, Ada and Ana",
        directMemberIds: [3, 4, 5],
        users: [userFixture(5, "Ana Lima")],
      });

      yield* directs.addMembers(20, [5]);

      expect(store.getState().sidebar.rows[20]?.directMemberIds).toEqual([3, 4, 5]);
      expect(store.getState().users[5]?.name).toBe("Ana Lima");
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});
