import { Effect } from "effect";
import { ApiClient } from "./client.ts";
import { NetworkError, ServerError } from "./errors.ts";

/** Classic's tour stamp (users/tours#update); PUT answers there too. */
export const TOUR_PATH = "/users/me/tour";

/**
 * `PATCH /users/me/tour`: the endpoint classic's tour calls when it is skipped or finished, which
 * stamps `tour_completed_at` and answers 204. It sits outside `/api/v1`, so it goes out as its
 * own same-origin fetch with the CSRF token the app's writes carry.
 */
export const completeTour = Effect.fn("api.completeTour")(function* () {
  const token = yield* ApiClient.use((client) => client.csrfToken);

  const headers =
    token === null
      ? { Accept: "application/json" }
      : { Accept: "application/json", "X-CSRF-Token": token };

  const response = yield* Effect.tryPromise({
    try: () => fetch(TOUR_PATH, { method: "PATCH", credentials: "same-origin", headers }),
    catch: (error) => new NetworkError({ message: String(error) }),
  });

  if (!response.ok) {
    return yield* new ServerError({
      status: response.status,
      message: `The server answered ${response.status}`,
    });
  }
});
