import { afterEach, describe, expect, it } from "@effect/vitest";
import { Effect } from "effect";
import { ServerError } from "../api/errors.ts";
import { FakeApi, roomDetailFixture, sidebarRowFixture } from "../api/testing.ts";
import { mutations, store } from "../store/store.ts";
import { create } from "./room-actions.ts";

describe("room creation keys", () => {
  afterEach(() => mutations.reset());

  it.effect("sends the caller's key, so a retry after a lost reply names the same attempt", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const row = sidebarRowFixture(30, "Once");
      const response = { room: row.room, detail: roomDetailFixture(30), row };
      let attempts = 0;

      yield* fake.route("POST /rooms", () =>
        Effect.gen(function* () {
          attempts += 1;

          if (attempts === 1) {
            return yield* new ServerError({ status: 503, message: "Lost response" });
          }

          return response;
        }),
      );

      const body = {
        type: "open",
        clientRoomId: "attempt-1",
        name: "Once",
        iconName: null,
      } as const;

      const failed = yield* Effect.result(create(body));

      expect(failed._tag).toBe("Failure");

      yield* create(body);

      const requests = yield* fake.requests;

      expect(requests.map((request) => request.body)).toEqual([body, body]);
      expect(store.getState().sidebar.rows[30]?.displayName).toBe("Once");
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});
