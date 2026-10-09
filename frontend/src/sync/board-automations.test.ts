import { afterEach, describe, expect, it } from "@effect/vitest";
import { Effect, Result } from "effect";
import { Forbidden, NotFound, Validation } from "../api/errors.ts";
import { FakeApi } from "../api/testing.ts";
import { mutations, store } from "../store/store.ts";
import { BOARD, boardAutomations } from "../test/board-fixtures.ts";
import * as boards from "./board-actions.ts";
import { ActionError, asAction } from "./run.ts";

const off = { nudgeAfterMinutes: null, escalateAfterMinutes: null };

const sla = {
  planned: { nudgeAfterMinutes: 1440, escalateAfterMinutes: 2880 },
  inProgress: off,
  blocked: off,
};

afterEach(() => mutations.reset());

describe("board automation actions", () => {
  it.effect("returns settings, merges every response's users and sends the wire requests", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      yield* fake.reply(`GET /rooms/${BOARD}/automations`, boardAutomations(7));
      expect(yield* boards.automations(BOARD)).toEqual(boardAutomations(7));
      expect(store.getState().users[7]?.id).toBe(7);
      yield* fake.reply(`POST /rooms/${BOARD}/automations/tag_rules`, boardAutomations(8));
      expect(yield* boards.addTagRule(BOARD, { tag: "Design", assigneeId: 8 })).toEqual(
        boardAutomations(8),
      );
      expect(store.getState().users[8]?.id).toBe(8);
      yield* fake.reply(`DELETE /rooms/${BOARD}/automations/tag_rules/1`, boardAutomations(9));
      expect(yield* boards.removeTagRule(BOARD, 1)).toEqual(boardAutomations(9));
      expect(store.getState().users[9]?.id).toBe(9);
      yield* fake.reply(`PUT /rooms/${BOARD}/automations/sla_timers`, boardAutomations(10));
      expect(yield* boards.saveSlaTimers(BOARD, sla)).toEqual(boardAutomations(10));
      expect(Object.keys(store.getState().users)).toEqual(["7", "8", "9", "10"]);
      expect(yield* fake.requests).toEqual([
        { method: "GET", path: `/rooms/${BOARD}/automations` },
        {
          method: "POST",
          path: `/rooms/${BOARD}/automations/tag_rules`,
          body: { tag: "Design", assigneeId: 8 },
        },
        { method: "DELETE", path: `/rooms/${BOARD}/automations/tag_rules/1` },
        { method: "PUT", path: `/rooms/${BOARD}/automations/sla_timers`, body: sla },
      ]);
      expect(store.getState().boards).toEqual({});
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("rejects with ActionError and preserves tag and assignee wire fields", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      const fields = { tag: ["has already been taken"], assigneeId: ["must exist"] };
      yield* fake.route(`POST /rooms/${BOARD}/automations/tag_rules`, () =>
        Effect.fail(
          new Validation({ message: "Tag has already been taken and Assignee must exist", fields }),
        ),
      );

      const result = yield* Effect.result(
        asAction(boards.addTagRule(BOARD, { tag: "bug", assigneeId: null })),
      );

      if (Result.isSuccess(result)) throw new Error("Expected validation failure");
      expect(result.failure).toBeInstanceOf(ActionError);
      expect(result.failure.tag).toBe("Validation");
      expect(result.failure.fields).toEqual(fields);
      expect(result.failure.message).toBe("Tag has already been taken and Assignee must exist");
      expect(store.getState().users).toEqual({});
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("preserves SLA row fields and the classic alert", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;

      const fields = {
        planned: ["Escalate after minutes must be after the nudge threshold"],
        inProgress: ["Nudge after minutes can't be blank"],
        blocked: ["Nudge after minutes must be greater than 0"],
      };

      const message = "Planned: Escalate after minutes must be after the nudge threshold";
      yield* fake.route(`PUT /rooms/${BOARD}/automations/sla_timers`, () =>
        Effect.fail(new Validation({ message, fields })),
      );
      const result = yield* Effect.result(asAction(boards.saveSlaTimers(BOARD, sla)));

      if (Result.isSuccess(result)) throw new Error("Expected validation failure");
      expect(result.failure).toBeInstanceOf(ActionError);
      expect(result.failure.fields).toEqual(fields);
      expect(result.failure.message).toBe(message);
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );

  it.effect("rejects read and delete permission failures without swallowing them", () =>
    Effect.gen(function* () {
      const fake = yield* FakeApi;
      yield* fake.route(`GET /rooms/${BOARD}/automations`, () =>
        Effect.fail(new Forbidden({ message: "Forbidden" })),
      );
      yield* fake.route(`DELETE /rooms/${BOARD}/automations/tag_rules/999`, () =>
        Effect.fail(new NotFound({ message: "Rule not found." })),
      );
      const read = yield* Effect.result(asAction(boards.automations(BOARD)));
      const removed = yield* Effect.result(asAction(boards.removeTagRule(BOARD, 999)));
      expect(Result.isFailure(read) ? read.failure : null).toMatchObject({
        tag: "Forbidden",
        message: "Forbidden",
        fields: {},
      });
      expect(Result.isFailure(removed) ? removed.failure : null).toMatchObject({
        tag: "NotFound",
        message: "Rule not found.",
        fields: {},
      });
    }).pipe(Effect.provide(FakeApi.layerClient)),
  );
});
