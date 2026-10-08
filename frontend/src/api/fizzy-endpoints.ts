import { Effect } from "effect";
import type { CreatedFizzyCard } from "../gen/CreatedFizzyCard.ts";
import type { CreateFizzyCard } from "../gen/CreateFizzyCard.ts";
import type { FizzyMessageCardForm } from "../gen/FizzyMessageCardForm.ts";
import { call, get } from "./call.ts";
import {
  CreatedFizzyCard as CreatedFizzyCardSchema,
  FizzyMessageCardForm as FizzyMessageCardFormSchema,
} from "./schema/fizzy.ts";
import { wire } from "./wire.ts";

/** Use the message's room and thread ids. A room timeline always has threadId null. */
export interface FizzyMessageScope {
  readonly roomId: number;
  readonly threadId: number | null;
  readonly messageId: number;
}

function path(scope: FizzyMessageScope): string {
  const conversation = scope.threadId === null ? "" : `/threads/${scope.threadId}`;

  return `/rooms/${scope.roomId}${conversation}/messages/${scope.messageId}/fizzy_cards`;
}

/** A disconnected viewer gets the source and defaults with connected false and no boards. */
export const fizzyMessageCardForm = Effect.fn("api.fizzyMessageCardForm")(function* (
  scope: FizzyMessageScope,
) {
  return yield* call(
    get(`${path(scope)}/new`),
    wire<FizzyMessageCardForm>(FizzyMessageCardFormSchema),
  );
});

/** Creates once, without retrying network failures. Classic creation is not idempotent. */
export const createFizzyMessageCard = Effect.fn("api.createFizzyMessageCard")(function* (
  scope: FizzyMessageScope,
  body: CreateFizzyCard,
) {
  return yield* call(
    { method: "POST", path: path(scope), body: { ...body } },
    wire<CreatedFizzyCard>(CreatedFizzyCardSchema),
  );
});
