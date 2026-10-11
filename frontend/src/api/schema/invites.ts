import { Schema } from "effect";
import type { CreateWorkspaceInvite as GeneratedCreateWorkspaceInvite } from "../../gen/CreateWorkspaceInvite.ts";
import type { WorkspaceInvite as GeneratedWorkspaceInvite } from "../../gen/WorkspaceInvite.ts";
import type { WorkspaceInviteCreated as GeneratedWorkspaceInviteCreated } from "../../gen/WorkspaceInviteCreated.ts";
import type { WorkspaceInviteCreator as GeneratedWorkspaceInviteCreator } from "../../gen/WorkspaceInviteCreator.ts";
import type { WorkspaceInviteExpiry as GeneratedWorkspaceInviteExpiry } from "../../gen/WorkspaceInviteExpiry.ts";
import type { WorkspaceInviteList as GeneratedWorkspaceInviteList } from "../../gen/WorkspaceInviteList.ts";
import type { WorkspaceInviteState as GeneratedWorkspaceInviteState } from "../../gen/WorkspaceInviteState.ts";
import { UserId, WorkspaceInviteId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Timestamp } from "./time.ts";

/** How long a new invite lasts, as Discord offers it. */
export const WorkspaceInviteExpiry = Schema.Literals([
  "30m",
  "1h",
  "6h",
  "12h",
  "1d",
  "7d",
  "never",
]);

export type WorkspaceInviteExpiryPin = Assert<
  Pinned<typeof WorkspaceInviteExpiry, GeneratedWorkspaceInviteExpiry>
>;

/** Where an invite stands; revoked wins over expired, and expired over used up. */
export const WorkspaceInviteState = Schema.Literals(["active", "expired", "exhausted", "revoked"]);

export type WorkspaceInviteStatePin = Assert<
  Pinned<typeof WorkspaceInviteState, GeneratedWorkspaceInviteState>
>;

/** `POST /admin/invites`: `null` maxUses is no limit. */
export const CreateWorkspaceInvite = Schema.Struct({
  expiry: WorkspaceInviteExpiry,
  maxUses: Schema.NullOr(Schema.Int),
});

export type CreateWorkspaceInvitePin = Assert<
  Pinned<typeof CreateWorkspaceInvite, GeneratedCreateWorkspaceInvite>
>;

export const WorkspaceInviteCreator = Schema.Struct({
  id: UserId,
  name: Schema.String,
});

export type WorkspaceInviteCreatorPin = Assert<
  Pinned<typeof WorkspaceInviteCreator, GeneratedWorkspaceInviteCreator>
>;

/** An invite without its token, which is only ever shown once. */
export const WorkspaceInvite = Schema.Struct({
  id: WorkspaceInviteId,
  creator: WorkspaceInviteCreator,
  expiresAt: Schema.NullOr(Timestamp),
  maxUses: Schema.NullOr(Schema.Int),
  uses: Schema.Int,
  revokedAt: Schema.NullOr(Timestamp),
  createdAt: Timestamp,
  updatedAt: Timestamp,
  state: WorkspaceInviteState,
});

export type WorkspaceInvitePin = Assert<Pinned<typeof WorkspaceInvite, GeneratedWorkspaceInvite>>;

/** A new invite, with its token and shareable URL. */
export const WorkspaceInviteCreated = Schema.Struct({
  invite: WorkspaceInvite,
  token: Schema.String,
  url: Schema.String,
});

export type WorkspaceInviteCreatedPin = Assert<
  Pinned<typeof WorkspaceInviteCreated, GeneratedWorkspaceInviteCreated>
>;

/** `GET /admin/invites`, newest first. */
export const WorkspaceInviteList = Schema.Struct({
  invites: Schema.Array(WorkspaceInvite),
});

export type WorkspaceInviteListPin = Assert<
  Pinned<typeof WorkspaceInviteList, GeneratedWorkspaceInviteList>
>;
