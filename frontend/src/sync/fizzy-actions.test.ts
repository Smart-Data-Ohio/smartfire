import { afterEach, describe, expect, it } from "@effect/vitest";
import { Effect } from "effect";
import { FizzyReadOnly, FizzyReplyFailed } from "../api/errors.ts";
import { FakeApi, meFixture, messageFixture, pageFixture } from "../api/testing.ts";
import { mutations, store } from "../store/store.ts";
import * as fizzy from "./fizzy-actions.ts";
import { asAction } from "./run.ts";

const body = { boardId: "engineering", title: "Fix it", description: "Details" };

describe("Fizzy dialog actions", () => {
  afterEach(() => mutations.reset());

  for (const threadId of [null, 40]) {
    it.effect(`lands the reply on the ${threadId === null ? "room" : "thread"} timeline`, () =>
      Effect.gen(function* () {
        mutations.reset();
        mutations.setMe(meFixture);
        const source = messageFixture(1, 12, { threadId });
        const page = pageFixture([source]);

        if (threadId === null) mutations.applyPage(12, page, "replace");
        else mutations.applyThreadPage(threadId, page, "replace");

        const reply = messageFixture(2, 12, { threadId, replyToMessageId: 1 });

        const created = {
          number: "580",
          url: "https://fizzy.test/cards/580",
          notice: "Fizzy card #580 created.",
          message: reply,
        };

        const fake = yield* FakeApi;
        const path = `/rooms/12${threadId === null ? "" : `/threads/${threadId}`}/messages/1/fizzy_cards`;
        yield* fake.route(`POST ${path}`, () => Effect.succeed(created));
        expect(yield* fizzy.create({ roomId: 12, threadId, messageId: 1 }, body)).toEqual(created);

        const timeline =
          threadId === null
            ? store.getState().timelines[12]
            : store.getState().threadTimelines[threadId];

        expect(timeline?.ids).toEqual([1, 2]);
        expect(store.getState().messages[2]).toEqual(reply);
        expect((yield* fake.requests).at(-1)?.body).toEqual(body);
      }).pipe(Effect.provide(FakeApi.layerClient)),
    );
  }

  it.effect("keeps typed errors available to the dialog and never retries creation", () =>
    Effect.gen(function* () {
      mutations.reset();
      const fake = yield* FakeApi;
      yield* fake.route("POST /rooms/12/messages/1/fizzy_cards", () =>
        Effect.fail(new FizzyReadOnly({ message: "That Fizzy token is read-only." })),
      );

      const error = yield* Effect.flip(
        asAction(fizzy.create({ roomId: 12, threadId: null, messageId: 1 }, body)),
      );

      expect(error.tag).toBe("FizzyReadOnly");
      expect(error.message).toBe("That Fizzy token is read-only.");
      expect(yield* fake.requests).toHaveLength(1);
      expect(store.getState().messages).toEqual({});
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
  it.effect("preserves the existing card when posting its reply failed", () =>
    Effect.gen(function* () {
      const error = yield* Effect.flip(
        asAction(
          Effect.fail(
            new FizzyReplyFailed({
              message:
                "Fizzy card #580 created, but the reply could not be posted (Body is too long).",
              number: "580",
              url: "https://fizzy.test/cards/580",
            }),
          ),
        ),
      );

      expect(error.tag).toBe("FizzyReplyFailed");
      expect(error.createdCard).toEqual({ number: "580", url: "https://fizzy.test/cards/580" });
    }),
  );
});
