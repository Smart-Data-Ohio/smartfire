import { Effect } from "effect";
import type { BoardListing } from "../gen/BoardListing.ts";
import type { BoardPostForm } from "../gen/BoardPostForm.ts";
import type { BoardStatusFilter } from "../gen/BoardStatusFilter.ts";
import type { CreateBoardPost } from "../gen/CreateBoardPost.ts";
import type { CreateWorkHandoff } from "../gen/CreateWorkHandoff.ts";
import type { ThreadDetail } from "../gen/ThreadDetail.ts";
import type { UpdateWork } from "../gen/UpdateWork.ts";
import type { WorkFilter } from "../gen/WorkFilter.ts";
import type { WorkList } from "../gen/WorkList.ts";
import { call, get } from "./call.ts";
import {
  BoardListing as BoardListingSchema,
  BoardPostForm as BoardPostFormSchema,
} from "./schema/board.ts";
import { ThreadDetail as ThreadDetailSchema } from "./schema/thread.ts";
import { WorkList as WorkListSchema } from "./schema/work.ts";
import { wire } from "./wire.ts";

export const board = Effect.fn("api.board")(function* (
  roomId: number,
  query: {
    readonly status: BoardStatusFilter;
    readonly owner: string;
    readonly tag: string;
    readonly page: number;
  },
) {
  return yield* call(
    get(`/rooms/${roomId}/board`, { ...query, page: String(query.page) }),
    wire<BoardListing>(BoardListingSchema),
  );
});

export const boardPostForm = Effect.fn("api.boardPostForm")(function* (roomId: number) {
  return yield* call(get(`/rooms/${roomId}/posts/new`), wire<BoardPostForm>(BoardPostFormSchema));
});

export const createBoardPost = Effect.fn("api.createBoardPost")(function* (
  roomId: number,
  body: CreateBoardPost,
) {
  return yield* call(
    { method: "POST", path: `/rooms/${roomId}/posts`, body },
    wire<ThreadDetail>(ThreadDetailSchema),
  );
});

export const updateWork = Effect.fn("api.updateWork")(function* (
  threadId: number,
  body: UpdateWork,
) {
  return yield* call(
    { method: "PATCH", path: `/threads/${threadId}/work`, body },
    wire<ThreadDetail>(ThreadDetailSchema),
  );
});

export const handoffWork = Effect.fn("api.handoffWork")(function* (
  threadId: number,
  body: CreateWorkHandoff,
) {
  return yield* call(
    { method: "POST", path: `/threads/${threadId}/work/handoff`, body },
    wire<ThreadDetail>(ThreadDetailSchema),
  );
});

export const workList = Effect.fn("api.workList")(function* (state: WorkFilter) {
  return yield* call(get("/work", { state }), wire<WorkList>(WorkListSchema));
});
