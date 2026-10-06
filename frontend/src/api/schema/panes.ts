import { Schema } from "effect";
import type { FileList as GeneratedFileList } from "../../gen/FileList.ts";
import type { FileType as GeneratedFileType } from "../../gen/FileType.ts";
import type { Member as GeneratedMember } from "../../gen/Member.ts";
import type { MemberList as GeneratedMemberList } from "../../gen/MemberList.ts";
import type { RoomFile as GeneratedRoomFile } from "../../gen/RoomFile.ts";
import type { StarState as GeneratedStarState } from "../../gen/StarState.ts";
import { Attachment } from "./attachment.ts";
import { MessageId, ThreadId, UserId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Presence } from "./presence.ts";
import { Timestamp } from "./time.ts";
import { User } from "./user.ts";

export const Member = Schema.Struct({
  userId: UserId,
  presence: Presence,
  statusText: Schema.NullOr(Schema.String),
  starred: Schema.Boolean,
});

export type Member = typeof Member.Type;

export type MemberPin = Assert<Pinned<typeof Member, GeneratedMember>>;

/** `GET /api/v1/rooms/:id/members`: every active member, by name. */
export const MemberList = Schema.Struct({
  members: Schema.Array(Member),
  users: Schema.Array(User),
});

export type MemberList = typeof MemberList.Type;

export type MemberListPin = Assert<Pinned<typeof MemberList, GeneratedMemberList>>;

/** The reply to `PUT` / `DELETE /api/v1/users/:id/star`. */
export const StarState = Schema.Struct({ userId: UserId, starred: Schema.Boolean });

export type StarState = typeof StarState.Type;

export type StarStatePin = Assert<Pinned<typeof StarState, GeneratedStarState>>;

export const FileType = Schema.Literals(["all", "images", "videos", "documents", "other"]);

export type FileType = typeof FileType.Type;

export type FileTypePin = Assert<Pinned<typeof FileType, GeneratedFileType>>;

export const RoomFile = Schema.Struct({
  messageId: MessageId,
  threadId: Schema.NullOr(ThreadId),
  creatorId: UserId,
  attachment: Attachment,
  createdAt: Timestamp,
});

export type RoomFile = typeof RoomFile.Type;

export type RoomFilePin = Assert<Pinned<typeof RoomFile, GeneratedRoomFile>>;

/** `GET /api/v1/rooms/:id/files?type=&filename=&page=`: newest first, 30 a page. */
export const FileList = Schema.Struct({
  files: Schema.Array(RoomFile),
  users: Schema.Array(User),
  nextPage: Schema.NullOr(Schema.Int),
});

export type FileList = typeof FileList.Type;

export type FileListPin = Assert<Pinned<typeof FileList, GeneratedFileList>>;
