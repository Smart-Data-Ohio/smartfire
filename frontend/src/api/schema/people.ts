import { Schema } from "effect";
import type { BlockedPerson as GeneratedBlockedPerson } from "../../gen/BlockedPerson.ts";
import type { BlockList as GeneratedBlockList } from "../../gen/BlockList.ts";
import type { BlockState as GeneratedBlockState } from "../../gen/BlockState.ts";
import type { DirectoryPerson as GeneratedDirectoryPerson } from "../../gen/DirectoryPerson.ts";
import type { PeopleDirectory as GeneratedPeopleDirectory } from "../../gen/PeopleDirectory.ts";
import type { PersonProfile as GeneratedPersonProfile } from "../../gen/PersonProfile.ts";
import type { PersonStatus as GeneratedPersonStatus } from "../../gen/PersonStatus.ts";
import { UserId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Presence } from "./presence.ts";
import { Timestamp } from "./time.ts";
import { User } from "./user.ts";

/** A row of the classic people directory (`users#index`). */
export const DirectoryPerson = Schema.Struct({
  userId: UserId,
  ignoredByMe: Schema.Boolean,
  online: Schema.Boolean,
  starred: Schema.Boolean,
  agent: Schema.Boolean,
});

export type DirectoryPerson = typeof DirectoryPerson.Type;

export type DirectoryPersonPin = Assert<Pinned<typeof DirectoryPerson, GeneratedDirectoryPerson>>;

/** `GET /api/v1/people`: everyone active but the viewer, starred first, then by name. */
export const PeopleDirectory = Schema.Struct({
  people: Schema.Array(DirectoryPerson),
  users: Schema.Array(User),
});

export type PeopleDirectory = typeof PeopleDirectory.Type;

export type PeopleDirectoryPin = Assert<Pinned<typeof PeopleDirectory, GeneratedPeopleDirectory>>;

/** A person page's status badge: their presence and status line. */
export const PersonStatus = Schema.Struct({
  presence: Presence,
  statusText: Schema.NullOr(Schema.String),
});

export type PersonStatus = typeof PersonStatus.Type;

export type PersonStatusPin = Assert<Pinned<typeof PersonStatus, GeneratedPersonStatus>>;

/** `GET /api/v1/people/:id` (and each ban change's answer): what the classic page shows. */
export const PersonProfile = Schema.Struct({
  user: User,
  ignoredByMe: Schema.Boolean,
  status: Schema.NullOr(PersonStatus),
  dndAllowed: Schema.NullOr(Schema.Boolean),
  emailAddress: Schema.NullOr(Schema.String),
  transferUrl: Schema.NullOr(Schema.String),
  transferQrSvg: Schema.NullOr(Schema.String),
  canBan: Schema.Boolean,
  canManageBot: Schema.Boolean,
});

export type PersonProfile = typeof PersonProfile.Type;

export type PersonProfilePin = Assert<Pinned<typeof PersonProfile, GeneratedPersonProfile>>;

export const BlockedPerson = Schema.Struct({
  user: User,
  ignoredByMe: Schema.Boolean,
  createdAt: Timestamp,
});

export type BlockedPersonPin = Assert<Pinned<typeof BlockedPerson, GeneratedBlockedPerson>>;

export const BlockList = Schema.Struct({
  viewerId: UserId,
  people: Schema.Array(BlockedPerson),
  revision: Schema.Int,
});

export type BlockListPin = Assert<Pinned<typeof BlockList, GeneratedBlockList>>;

export const BlockState = Schema.Struct({
  viewerId: UserId,
  userId: UserId,
  ignoredByMe: Schema.Boolean,
  revision: Schema.Int,
});

export type BlockStatePin = Assert<Pinned<typeof BlockState, GeneratedBlockState>>;
