/**
 * The agent screens' loads as Effect programs (S4). Each lands in the store, failures too (as the
 * list's or profile's error), so the loads never fail. Generations protect page membership
 * and profile content; status records compare server revisions independently.
 */
import { Effect, Predicate } from "effect";
import * as api from "../api/agent-endpoints.ts";
import { directoryGeneration, profileOf } from "../store/agents.ts";
import { mutations, store } from "../store/store.ts";

/** Loads (or reloads) the directory; the rows shown stay while it reloads. */
export const loadDirectory = Effect.fn("agents.loadDirectory")(function* () {
  mutations.setAgentDirectoryLoading();

  const generation = directoryGeneration(store.getState());

  yield* api.agentDirectory().pipe(
    Effect.tap((page) => Effect.sync(() => mutations.landAgentDirectory(page, generation))),
    Effect.catch((error) =>
      Effect.sync(() => mutations.setAgentDirectoryFailed(error.message, generation)),
    ),
  );
});

/** Loads (or reloads) an agent's profile; a 404 marks it missing. */
export const loadProfile = Effect.fn("agents.loadProfile")(function* (agentId: number) {
  mutations.setAgentProfileLoading(agentId);

  const generation = profileOf(store.getState(), agentId).generation;

  yield* api.agentProfile(agentId).pipe(
    Effect.tap((profile) => Effect.sync(() => mutations.landAgentProfile(profile, generation))),
    Effect.catch((error) =>
      Effect.sync(() =>
        mutations.setAgentProfileFailed(
          agentId,
          error.message,
          Predicate.isTagged(error, "NotFound"),
          generation,
        ),
      ),
    ),
  );
});
