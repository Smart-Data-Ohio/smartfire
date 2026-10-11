/**
 * The S7 people endpoints: the classic directory (`users#index`), a person's page (`users#show`)
 * and its ban button. Banning needs an administrator and waits for a fresh confirmation once the
 * last one has lapsed.
 */
import { Effect } from "effect";
import type { BlockList } from "../gen/BlockList.ts";
import type { BlockState } from "../gen/BlockState.ts";
import type { PeopleDirectory } from "../gen/PeopleDirectory.ts";
import type { PersonProfile } from "../gen/PersonProfile.ts";
import { call, get } from "./call.ts";
import {
  BlockList as BlockListSchema,
  BlockState as BlockStateSchema,
  PeopleDirectory as PeopleDirectorySchema,
  PersonProfile as PersonProfileSchema,
} from "./schema/people.ts";
import { wire } from "./wire.ts";

/** `GET /people`: everyone active but the viewer, starred first, then by name. */
export const peopleDirectory = Effect.fn("api.peopleDirectory")(function* () {
  return yield* call(get("/people"), wire<PeopleDirectory>(PeopleDirectorySchema));
});

const profileReply = wire<PersonProfile>(PersonProfileSchema);

/** `GET /people/:id`: what the classic page shows this viewer about them. */
export const personProfile = Effect.fn("api.personProfile")(function* (userId: number) {
  return yield* call(get(`/people/${userId}`), profileReply);
});

/** `POST` (ban) or `DELETE` (remove the ban) `/people/:id/ban`: their page as it now stands. */
export const setBanned = Effect.fn("api.setBanned")(function* (userId: number, banned: boolean) {
  return yield* call(
    { method: banned ? "POST" : "DELETE", path: `/people/${userId}/ban` },
    profileReply,
  );
});

export const blocks = Effect.fn("api.blocks")(function* () {
  return yield* call(get("/blocks"), wire<BlockList>(BlockListSchema));
});

export const setIgnored = Effect.fn("api.setIgnored")(function* (userId: number, ignored: boolean) {
  return yield* call(
    { method: ignored ? "PUT" : "DELETE", path: `/people/${userId}/block` },
    wire<BlockState>(BlockStateSchema),
  );
});
