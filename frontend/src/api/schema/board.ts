import { Schema } from "effect";
import type { BoardDigest as GeneratedBoardDigest } from "../../gen/BoardDigest.ts";
import type { BoardListing as GeneratedBoardListing } from "../../gen/BoardListing.ts";
import type { BoardOwnerOption as GeneratedBoardOwnerOption } from "../../gen/BoardOwnerOption.ts";
import type { BoardPostForm as GeneratedBoardPostForm } from "../../gen/BoardPostForm.ts";
import type { BoardStatusFilter as GeneratedBoardStatusFilter } from "../../gen/BoardStatusFilter.ts";
import type { BoardTagCount as GeneratedBoardTagCount } from "../../gen/BoardTagCount.ts";
import type { CreateBoardPost as GeneratedCreateBoardPost } from "../../gen/CreateBoardPost.ts";
import { RoomId, UserId } from "./ids.ts";
import { CreateMessage } from "./message.ts";
import type { Assert, Pinned } from "./pin.ts";
import { ThreadSummary } from "./thread.ts";
import { User } from "./user.ts";
import { WorkOwnerCandidate, WorkStatus } from "./work-parts.ts";

export const BoardStatusFilter = Schema.Literals(["open", "done", "all"]);

export type BoardStatusFilter = typeof BoardStatusFilter.Type;

export type BoardStatusFilterPin = Assert<
  Pinned<typeof BoardStatusFilter, GeneratedBoardStatusFilter>
>;

export const BoardOwnerOption = Schema.Struct({ userId: UserId, agent: Schema.Boolean });

export type BoardOwnerOption = typeof BoardOwnerOption.Type;

export type BoardOwnerOptionPin = Assert<
  Pinned<typeof BoardOwnerOption, GeneratedBoardOwnerOption>
>;

export const BoardTagCount = Schema.Struct({ name: Schema.String, count: Schema.Int });

export type BoardTagCount = typeof BoardTagCount.Type;

export type BoardTagCountPin = Assert<Pinned<typeof BoardTagCount, GeneratedBoardTagCount>>;

export const BoardDigest = Schema.Struct({ date: Schema.String, text: Schema.String });

export type BoardDigest = typeof BoardDigest.Type;

export type BoardDigestPin = Assert<Pinned<typeof BoardDigest, GeneratedBoardDigest>>;

/** The server's cumulative window: page n holds the first n × 50 matches. */
export const BoardListing = Schema.Struct({
  roomId: RoomId,
  status: BoardStatusFilter,
  owner: Schema.String,
  tag: Schema.String,
  page: Schema.Int,
  posts: Schema.Array(ThreadSummary),
  hasMore: Schema.Boolean,
  anyPosts: Schema.Boolean,
  ownerOptions: Schema.Array(BoardOwnerOption),
  tagCounts: Schema.Array(BoardTagCount),
  digest: Schema.NullOr(BoardDigest),
  canAdminister: Schema.Boolean,
  users: Schema.Array(User),
});

export type BoardListing = typeof BoardListing.Type;

export type BoardListingPin = Assert<Pinned<typeof BoardListing, GeneratedBoardListing>>;

export const BoardPostForm = Schema.Struct({
  ownerCandidates: Schema.Array(WorkOwnerCandidate),
  tagSuggestions: Schema.Array(Schema.String),
  users: Schema.Array(User),
});

export type BoardPostForm = typeof BoardPostForm.Type;

export type BoardPostFormPin = Assert<Pinned<typeof BoardPostForm, GeneratedBoardPostForm>>;

export const CreateBoardPost = Schema.Struct({
  name: Schema.String,
  status: WorkStatus,
  ownerId: Schema.NullOr(UserId),
  tags: Schema.Array(Schema.String),
  message: Schema.NullOr(CreateMessage),
});

export type CreateBoardPost = typeof CreateBoardPost.Type;

export type CreateBoardPostPin = Assert<Pinned<typeof CreateBoardPost, GeneratedCreateBoardPost>>;
