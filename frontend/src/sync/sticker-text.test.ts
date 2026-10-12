import { afterEach, describe, expect, it } from "@effect/vitest";
import { Effect } from "effect";
import { expectStatus, harness, NOW, send } from "../../mock/s2/testing.ts";
import { SEED_IDS } from "../../mock/server.ts";
import { FakeApi } from "../api/testing.ts";
import type { MessageDTO } from "../gen/MessageDTO.ts";
import { mutations, store } from "../store/store.ts";
import * as activity from "./activity-actions.ts";
import * as fizzy from "./fizzy-actions.ts";
import * as threads from "./thread-actions.ts";

describe("sticker preview display text", () => {
  afterEach(() => mutations.reset());

  for (const deleted of [false, true]) {
    it.effect(
      `loads the retained sticker name into ${deleted ? "deleted" : "live"} sticker previews`,
      () =>
        Effect.gen(function* () {
          const { server } = harness();

          const message = yield* Effect.promise(() =>
            expectStatus<MessageDTO>(
              server,
              "POST",
              `/api/v1/rooms/${SEED_IDS.rooms.lounge}/messages`,
              { clientMessageId: "sticker-reminder", markdownSource: "Caption", stickerId: 2 },
              201,
            ),
          );

          yield* Effect.promise(() =>
            expectStatus(
              server,
              "PATCH",
              `/api/v1/messages/${message.id}`,
              { markdownSource: "" },
              200,
            ),
          );

          if (deleted) {
            yield* Effect.promise(() =>
              expectStatus(server, "DELETE", "/api/v1/admin/stickers/2", null, 200),
            );
          }

          yield* Effect.promise(() =>
            expectStatus(
              server,
              "POST",
              "/api/v1/saved",
              { messageId: message.id, remindAt: new Date(NOW + 60_000).toISOString() },
              201,
            ),
          );
          yield* Effect.promise(() => send(server, "POST", "/__mock/remind-due", { all: true }));

          const fake = yield* FakeApi;

          const roomId = message.roomId;
          const formPath = `/rooms/${roomId}/messages/${message.id}/fizzy_cards/new`;

          yield* fake.route(`GET ${formPath}`, () =>
            Effect.promise(() => server.handle({ method: "GET", path: `/api/v1${formPath}` })).pipe(
              Effect.map((response) => {
                expect(response.status).toBe(200);

                return response.json;
              }),
            ),
          );
          yield* fake.route(`POST /rooms/${roomId}/threads`, (request) =>
            Effect.promise(() =>
              send(server, "POST", `/api/v1/rooms/${roomId}/threads`, request.body),
            ).pipe(
              Effect.map((response) => {
                expect(response.status).toBe(201);

                return response.json;
              }),
            ),
          );

          const form = yield* fizzy.form({ roomId, threadId: null, messageId: message.id });
          const threadId = yield* threads.create(roomId, message.id, "Reply");

          yield* fake.route("GET /activity", () =>
            Effect.promise(() =>
              server.handle({
                method: "GET",
                path: "/api/v1/activity?type=reminders&status=unread",
              }),
            ).pipe(
              Effect.map((response) => {
                expect(response.status).toBe(200);

                return response.json;
              }),
            ),
          );
          yield* activity.load("reminders", "unread");

          const reminder = Object.values(store.getState().activity.items).find(
            (item) => item?.source.messageId === message.id,
          );

          expect({
            reminder: reminder?.source.body,
            threadName: store.getState().threads[threadId]?.name,
            fizzyTitle: form.title,
            fizzyExcerpt: form.excerpt,
          }).toEqual({
            reminder: "You asked to be reminded about this message: Sticker: Dance",
            threadName: "Sticker: Dance",
            fizzyTitle: "Sticker: Dance",
            fizzyExcerpt: "Sticker: Dance",
          });
        }).pipe(Effect.provide(FakeApi.layerClient)),
    );
  }
});
