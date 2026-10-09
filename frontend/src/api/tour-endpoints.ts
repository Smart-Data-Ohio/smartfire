import { Effect } from "effect";
import { call, noContent } from "./call.ts";

/** Classic's tour stamp (users/tours#update); PUT answers there too. */
export const TOUR_PATH = "/users/me/tour";

/**
 * `PATCH /users/me/tour`: the endpoint classic's tour calls when it is skipped or finished, which
 * stamps `tour_completed_at` and answers an empty 204. It sits outside `/api/v1`, so it goes out
 * from the origin's root, through the app's client like every other write.
 */
export const completeTour = Effect.fn("api.completeTour")(function* () {
  return yield* call({ method: "PATCH", path: TOUR_PATH, root: true }, noContent);
});
