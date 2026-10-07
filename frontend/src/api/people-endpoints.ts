/**
 * The S7 people endpoints: the classic directory (`users#index`), a person's page (`users#show`)
 * and its ban button. Banning needs an administrator and fails with `SudoRequired` once the
 * password confirmation has lapsed.
 */
import { Effect } from "effect";
import type { PeopleDirectory } from "../gen/PeopleDirectory.ts";
import type { PersonProfile } from "../gen/PersonProfile.ts";
import { call, get } from "./call.ts";
import {
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
