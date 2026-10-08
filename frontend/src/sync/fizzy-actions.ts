import { Effect } from "effect";
import {
  createFizzyMessageCard,
  type FizzyMessageScope,
  fizzyMessageCardForm,
} from "../api/fizzy-endpoints.ts";
import type { CreateFizzyCard } from "../gen/CreateFizzyCard.ts";
import { mutations } from "../store/store.ts";

export const form = fizzyMessageCardForm;

/** Land the reply through the same reducer as all posts, including replies in a thread. */
export const create = Effect.fn("cards.createFizzy")(function* (
  scope: FizzyMessageScope,
  body: CreateFizzyCard,
) {
  const created = yield* createFizzyMessageCard(scope, body);

  mutations.receiveMessage(created.message);

  return created;
});
