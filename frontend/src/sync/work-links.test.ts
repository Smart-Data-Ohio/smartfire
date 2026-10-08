import { afterEach, describe, expect, it } from "@effect/vitest";
import { Effect, Result } from "effect";
import { NotFound, Validation } from "../api/errors.ts";
import { FakeApi } from "../api/testing.ts";
import type { WorkLink } from "../gen/WorkLink.ts";
import { mutations, store } from "../store/store.ts";
import { boardDetail, boardThread } from "../test/board-fixtures.ts";
import * as work from "./board-actions.ts";
import { asAction } from "./run.ts";

const link: WorkLink = {
  id: 3,
  kind: "event",
  label: "Review",
  url: "/rooms/900/events/9",
  pullRequestState: null,
  title: null,
  eventStartsAt: "2026-10-08T14:00:00.000Z",
  eventTimeZone: "America/New_York",
  eventCancelled: false,
};

function linked() {
  const detail = boardDetail();

  if (detail.thread.work === null) throw new Error("Work missing");
  detail.thread.work.links = [link];

  return detail;
}

afterEach(() => mutations.reset());

describe("work link actions", () => {
  it.effect("returns the form, installs write details and returns them with exact requests", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;

      const form = {
        events: [
          { id: 9, title: "Review", startsAt: link.eventStartsAt, timeZone: "America/New_York" },
        ],
      };

      yield* fake.reply("GET /threads/1/work/links/new", form);
      expect(yield* work.linkForm(1)).toEqual(form);
      expect(store.getState().threadPanes).toEqual({});
      yield* fake.reply("POST /threads/1/work/links", linked());
      expect(yield* work.addLink(1, { kind: "event", eventId: 9 })).toEqual(linked());
      expect(store.getState().threads[1]?.work?.links).toEqual([link]);
      expect(store.getState().threadPanes[1]?.workFacts?.links).toEqual([link]);
      expect(store.getState().users[7]?.id).toBe(7);
      yield* fake.reply("DELETE /threads/1/work/links/3", boardDetail());
      expect(yield* work.removeLink(1, 3)).toEqual(boardDetail());
      expect(store.getState().threads[1]?.work?.links).toEqual([]);
      expect(yield* fake.requests).toEqual([
        { method: "GET", path: "/threads/1/work/links/new" },
        { method: "POST", path: "/threads/1/work/links", body: { kind: "event", eventId: 9 } },
        { method: "DELETE", path: "/threads/1/work/links/3" },
      ]);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("keeps a newer live thread by refetching after a stale write reply", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      mutations.loadThreadDetail(boardDetail(), 0);
      const newer = boardDetail(boardThread(1, "done"));
      yield* fake.route("POST /threads/1/work/links", () =>
        Effect.sync(() => {
          mutations.applyEvents(
            [{ seq: 1, topic: "room:900", type: "thread.updated", data: newer.thread }],
            0,
          );

          return linked();
        }),
      );
      yield* fake.reply("GET /threads/1", newer);
      expect(yield* work.addLink(1, { kind: "event", eventId: 9 })).toEqual(linked());
      expect(store.getState().threads[1]?.work?.status).toBe("done");
      expect(store.getState().threadPanes[1]?.workFacts).toEqual(newer.thread.work);
      expect((yield* fake.requests).at(-1)?.path).toBe("/threads/1");
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("preserves classic messages/fields and failures leave the store alone", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const message = "That is already linked to this work thread.";
      yield* fake.route("POST /threads/1/work/links", () =>
        Effect.fail(new Validation({ message, fields: { eventId: [message] } })),
      );
      yield* fake.route("DELETE /threads/1/work/links/999", () =>
        Effect.fail(new NotFound({ message: "Not found" })),
      );
      const add = yield* Effect.result(asAction(work.addLink(1, { kind: "event", eventId: 9 })));
      const remove = yield* Effect.result(asAction(work.removeLink(1, 999)));
      expect(Result.isFailure(add) ? add.failure : null).toMatchObject({
        tag: "Validation",
        message,
        fields: { eventId: [message] },
      });
      expect(Result.isFailure(remove) ? remove.failure.tag : null).toBe("NotFound");
      expect(store.getState().threads).toEqual({});
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});
