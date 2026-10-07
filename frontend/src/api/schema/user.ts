import { Schema } from "effect";
import type { CustomStatus as GeneratedCustomStatus } from "../../gen/CustomStatus.ts";
import type { User as GeneratedUser } from "../../gen/User.ts";
import type { UserList as GeneratedUserList } from "../../gen/UserList.ts";
import type { UserRole as GeneratedUserRole } from "../../gen/UserRole.ts";
import type { UserStatus as GeneratedUserStatus } from "../../gen/UserStatus.ts";
import { UserId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Timestamp } from "./time.ts";

export const UserRole = Schema.Literals(["member", "administrator", "bot"]);

export type UserRolePin = Assert<Pinned<typeof UserRole, GeneratedUserRole>>;

export const UserStatus = Schema.Literals(["active", "deactivated", "banned"]);

export type UserStatusPin = Assert<Pinned<typeof UserStatus, GeneratedUserStatus>>;

export const CustomStatus = Schema.Struct({
  emoji: Schema.NullOr(Schema.String),
  text: Schema.NullOr(Schema.String),
  expiresAt: Schema.NullOr(Timestamp),
});

export type CustomStatusPin = Assert<Pinned<typeof CustomStatus, GeneratedCustomStatus>>;

/** A person or bot as every viewer sees them. */
export const User = Schema.Struct({
  id: UserId,
  name: Schema.String,
  role: UserRole,
  status: UserStatus,
  bio: Schema.NullOr(Schema.String),
  avatarUrl: Schema.String,
  /** Whether they uploaded a picture; `false` means `avatarUrl` serves their initials. */
  hasAvatar: Schema.Boolean,
  customStatus: Schema.NullOr(CustomStatus),
  createdAt: Timestamp,
});

export type User = typeof User.Type;

export type UserPin = Assert<Pinned<typeof User, GeneratedUser>>;

/** `GET /api/v1/users?ids=`: directory entries for up to 100 ids, in id order. */
export const UserList = Schema.Struct({ users: Schema.Array(User) });

export type UserList = typeof UserList.Type;

export type UserListPin = Assert<Pinned<typeof UserList, GeneratedUserList>>;
