import { afterEach, describe, expect, it } from "@effect/vitest";
import { Deferred, Effect, Fiber } from "effect";
import { TestClock } from "effect/testing";
import { Forbidden, Validation } from "../api/errors.ts";
import { FakeApi, meFixture, messageFixture, pageFixture } from "../api/testing.ts";
import type { EventAttendance } from "../gen/EventAttendance.ts";
import type { Poll } from "../gen/Poll.ts";
import { attendanceKey, githubKey, shownAttendance } from "../store/cards.ts";
import { mutations, store } from "../store/store.ts";
import * as cards from "./card-actions.ts";

const ROOM = 12;

const at = (second: number) => new Date(Date.UTC(2026, 9, 6, 12, 0, second)).toISOString();

function poll(asOf: string, votes: readonly [number, number], mine: boolean): Poll {
  return {
    id: 40,
    messageId: 1,
    asOf,
    multiple: false,
    anonymous: false,
    closesAt: null,
    closedAt: null,
    closed: false,
    totalVotes: votes[0] + votes[1],
    options: [
      { id: 401, label: "Tea", votes: votes[0], voterIds: mine ? [meFixture.user.id] : [] },
      { id: 402, label: "Coffee", votes: votes[1], voterIds: [] },
    ],
  };
}

function seed(): void {
  mutations.reset();
  mutations.setMe(meFixture);
  mutations.applyPage(
    ROOM,
    pageFixture([messageFixture(1, ROOM, { poll: poll(at(1), [0, 1], false) })]),
    "replace",
  );
}

const pending = () => store.getState().cards.pendingVotes[40];

/** Event 3's attendance as the card shows it: the fetched one with any answer on its way. */
const shown = () => {
  const { cards: slice } = store.getState();

  return shownAttendance(
    slice.previews.attendance[attendanceKey(3)]?.value ?? null,
    slice.pendingAnswers[3],
  );
};

describe("card actions", () => {
  afterEach(() => mutations.reset());

  it.effect("show a vote at once, then land the reply's poll and ballot", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      let seen: readonly number[] | undefined;

      yield* fake.route("POST /rooms/12/polls/40/vote", () => {
        seen = pending();

        return Effect.succeed({ poll: poll(at(5), [1, 1], true), myOptionIds: [401] });
      });

      yield* cards.vote(ROOM, 40, [401]);

      expect(seen).toEqual([401]);
      expect(pending()).toBeUndefined();
      expect(store.getState().messages[1]?.poll?.totalVotes).toBe(2);
      expect(store.getState().cards.ballots[40]).toEqual({ myOptionIds: [401], asOf: at(5) });

      const requests = yield* fake.requests;

      expect(requests.at(-1)?.body).toEqual({ optionIds: [401] });
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("put a refused vote back and reject with the server's message", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;

      yield* fake.route("POST /rooms/12/polls/40/vote", () =>
        Effect.fail(new Validation({ message: "This poll is closed", fields: {} })),
      );

      const failure = yield* Effect.flip(cards.vote(ROOM, 40, [401]));

      expect(failure.message).toBe("This poll is closed");
      expect(pending()).toBeUndefined();
      expect(store.getState().messages[1]?.poll?.totalVotes).toBe(1);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("post a poll and land its question in the timeline", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      const question = messageFixture(2, ROOM, { poll: { ...poll(at(2), [0, 0], false), id: 41 } });

      yield* fake.reply("POST /rooms/12/polls", question);

      const body = {
        clientMessageId: "0199a000-0000-7000-8000-000000000001",
        question: "Lunch?",
        options: ["Tea", "Coffee"],
        multiple: false,
        anonymous: false,
        closesAt: null,
      };

      const posted = yield* cards.createPoll(ROOM, body);

      expect(posted.id).toBe(2);
      expect(store.getState().timelines[ROOM]?.ids).toEqual([1, 2]);
      expect((yield* fake.requests).at(-1)?.body).toEqual(body);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("record a failed results fetch", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;

      yield* fake.route("GET /rooms/12/polls/40", () =>
        Effect.fail(new Forbidden({ message: "Not a member" })),
      );

      yield* Effect.flip(cards.loadPoll(ROOM, 40));

      expect(store.getState().cards.pollLoads[40]).toBe("error");
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("ask again while the server is still fetching a pull request", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      let calls = 0;

      yield* fake.route("GET /rooms/12/github/pull_requests/9/card", () => {
        calls += 1;

        return Effect.succeed(calls < 3 ? { state: "loading" } : { state: "hidden" });
      });

      const key = githubKey(ROOM, 9, { messageId: 1 });
      const fiber = yield* Effect.forkChild(cards.loadGithub(ROOM, 9, { messageId: 1 }));

      yield* Effect.yieldNow;

      expect(store.getState().cards.previews.github[key]?.value).toEqual({ state: "loading" });

      yield* TestClock.adjust("2 seconds");
      yield* TestClock.adjust("4 seconds");
      yield* Fiber.join(fiber);

      expect(calls).toBe(3);
      expect(store.getState().cards.previews.github[key]?.value).toEqual({ state: "hidden" });
      expect((yield* fake.requests)[0]?.query).toEqual({ messageId: "1" });
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("show an event response at once and put it back when refused", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;

      const attendance: EventAttendance = {
        eventId: 3,
        response: null,
        goingCount: 1,
        maybeCount: 0,
        declinedCount: 0,
        respondable: true,
        canApplyToFuture: true,
      };

      yield* fake.reply("GET /rooms/12/events/3/attendance", attendance);
      yield* cards.loadAttendance(ROOM, 3);

      let seen: EventAttendance | null | undefined;

      yield* fake.route("PUT /rooms/12/events/3/attendance", () => {
        seen = shown();

        return Effect.fail(new Forbidden({ message: "Not allowed" }));
      });

      const failure = yield* Effect.flip(cards.respond(ROOM, 3, "going", true));

      expect(failure.message).toBe("Not allowed");
      expect(seen).toMatchObject({ response: "going", goingCount: 2 });
      expect(shown()).toEqual(attendance);
      expect(store.getState().cards.pendingAnswers[3]).toBeUndefined();

      expect((yield* fake.requests).at(-1)?.body).toEqual({
        response: "going",
        applyToFuture: true,
      });
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("put two refused event responses back to the server's answer, not to a guess", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      const gate = yield* Deferred.make<void>();

      const attendance: EventAttendance = {
        eventId: 3,
        response: null,
        goingCount: 1,
        maybeCount: 0,
        declinedCount: 0,
        respondable: true,
        canApplyToFuture: true,
      };

      yield* fake.reply("GET /rooms/12/events/3/attendance", attendance);
      yield* cards.loadAttendance(ROOM, 3);
      yield* fake.route("PUT /rooms/12/events/3/attendance", () =>
        Deferred.await(gate).pipe(
          Effect.andThen(Effect.fail(new Forbidden({ message: "Not allowed" }))),
        ),
      );

      const both = yield* Effect.forkChild(
        Effect.all(
          [cards.respond(ROOM, 3, "going", false), cards.respond(ROOM, 3, "maybe", false)],
          { concurrency: "unbounded", mode: "result" },
        ),
      );

      yield* Effect.yieldNow;
      yield* Effect.yieldNow;
      yield* Deferred.succeed(gate, undefined);
      yield* Fiber.await(both);

      expect(shown()).toEqual(attendance);
      expect(store.getState().cards.pendingAnswers[3]).toBeUndefined();
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("show a second answer at once while the first is still on its way", () =>
    Effect.gen(function* () {
      seed();

      const fake = yield* FakeApi;
      const gate = yield* Deferred.make<void>();

      const attendance: EventAttendance = {
        eventId: 3,
        response: null,
        goingCount: 1,
        maybeCount: 0,
        declinedCount: 0,
        respondable: true,
        canApplyToFuture: false,
      };

      yield* fake.reply("GET /rooms/12/events/3/attendance", attendance);
      yield* cards.loadAttendance(ROOM, 3);

      let calls = 0;

      yield* fake.route("PUT /rooms/12/events/3/attendance", () => {
        calls += 1;

        const response = calls === 1 ? "going" : "maybe";

        const reply: EventAttendance = {
          ...attendance,
          response,
          goingCount: response === "going" ? 2 : 1,
          maybeCount: response === "maybe" ? 1 : 0,
        };

        return Deferred.await(gate).pipe(Effect.as(reply));
      });

      const going = yield* Effect.forkChild(cards.respond(ROOM, 3, "going", false));

      yield* Effect.yieldNow;
      expect(shown()).toMatchObject({ response: "going", goingCount: 2 });
      expect(store.getState().cards.pendingAnswers[3]).toBeDefined();

      const maybe = yield* Effect.forkChild(cards.respond(ROOM, 3, "maybe", false));

      yield* Effect.yieldNow;

      // The second answer shows while the first request still holds the event's turn.
      expect(calls).toBe(1);
      expect(shown()).toMatchObject({ response: "maybe", goingCount: 1, maybeCount: 1 });

      yield* Deferred.succeed(gate, undefined);
      yield* Fiber.join(going);
      yield* Fiber.join(maybe);

      const sent = (yield* fake.requests).flatMap((request) =>
        request.method === "PUT" ? [request.body] : [],
      );

      expect(sent).toEqual([
        { response: "going", applyToFuture: false },
        { response: "maybe", applyToFuture: false },
      ]);
      expect(shown()).toMatchObject({ response: "maybe", maybeCount: 1 });
      expect(store.getState().cards.pendingAnswers[3]).toBeUndefined();
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});
