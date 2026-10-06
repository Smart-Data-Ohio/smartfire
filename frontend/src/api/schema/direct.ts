import { Schema } from "effect";
import type { AddDirectMembers as GeneratedAddDirectMembers } from "../../gen/AddDirectMembers.ts";
import type { CreateDirect as GeneratedCreateDirect } from "../../gen/CreateDirect.ts";
import type { DirectCandidate as GeneratedDirectCandidate } from "../../gen/DirectCandidate.ts";
import type { DirectCandidateList as GeneratedDirectCandidateList } from "../../gen/DirectCandidateList.ts";
import type { RenameDirect as GeneratedRenameDirect } from "../../gen/RenameDirect.ts";
import { UserId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { User } from "./user.ts";

export const DirectCandidate = Schema.Struct({
  userId: UserId,
  agent: Schema.Boolean,
  starred: Schema.Boolean,
});

export type DirectCandidate = typeof DirectCandidate.Type;

export type DirectCandidatePin = Assert<Pinned<typeof DirectCandidate, GeneratedDirectCandidate>>;

/** `GET /api/v1/directs/candidates`: everyone but the viewer, starred first. */
export const DirectCandidateList = Schema.Struct({
  candidates: Schema.Array(DirectCandidate),
  users: Schema.Array(User),
});

export type DirectCandidateList = typeof DirectCandidateList.Type;

export type DirectCandidateListPin = Assert<
  Pinned<typeof DirectCandidateList, GeneratedDirectCandidateList>
>;

/** The body of `POST /api/v1/directs`: the other people (at most 9); answers a `SidebarRow`. */
export const CreateDirect = Schema.Struct({ userIds: Schema.Array(UserId) });

export type CreateDirect = typeof CreateDirect.Type;

export type CreateDirectPin = Assert<Pinned<typeof CreateDirect, GeneratedCreateDirect>>;

/** The body of `POST /api/v1/directs/:id/members`; answers the `RoomDetail`. */
export const AddDirectMembers = Schema.Struct({ userIds: Schema.Array(UserId) });

export type AddDirectMembers = typeof AddDirectMembers.Type;

export type AddDirectMembersPin = Assert<
  Pinned<typeof AddDirectMembers, GeneratedAddDirectMembers>
>;

/** The body of `PATCH /api/v1/directs/:id`; blank or `null` clears the name. */
export const RenameDirect = Schema.Struct({ name: Schema.NullOr(Schema.String) });

export type RenameDirect = typeof RenameDirect.Type;

export type RenameDirectPin = Assert<Pinned<typeof RenameDirect, GeneratedRenameDirect>>;
