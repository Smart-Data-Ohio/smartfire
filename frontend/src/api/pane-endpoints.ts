/** The S2 side panes' endpoints (members and files; pins are in message-endpoints.ts). */
import { Effect } from "effect";
import type { FileList } from "../gen/FileList.ts";
import type { FileType } from "../gen/FileType.ts";
import type { MemberList } from "../gen/MemberList.ts";
import type { StarState } from "../gen/StarState.ts";
import { call, get } from "./call.ts";
import {
  FileList as FileListSchema,
  MemberList as MemberListSchema,
  StarState as StarStateSchema,
} from "./schema/panes.ts";
import { wire } from "./wire.ts";

/** `GET /rooms/:id/members`: every active member with presence and the viewer's stars. */
export const members = Effect.fn("api.members")(function* (roomId: number) {
  return yield* call(get(`/rooms/${roomId}/members`), wire<MemberList>(MemberListSchema));
});

/** `PUT` (star) or `DELETE` (unstar) `/users/:id/star`. */
export const setStarred = Effect.fn("api.setStarred")(function* (userId: number, starred: boolean) {
  return yield* call(
    { method: starred ? "PUT" : "DELETE", path: `/users/${userId}/star` },
    wire<StarState>(StarStateSchema),
  );
});

export interface FileQuery {
  readonly type: FileType;
  /** Matches part of a filename; empty for all. */
  readonly filename: string;
  /** 1-based. */
  readonly page: number;
}

/** `GET /rooms/:id/files?type=&filename=&page=`: newest first, a page at a time. */
export const files = Effect.fn("api.files")(function* (roomId: number, query: FileQuery) {
  const base = { type: query.type, page: String(query.page) };
  const params = query.filename === "" ? base : { ...base, filename: query.filename };

  return yield* call(get(`/rooms/${roomId}/files`, params), wire<FileList>(FileListSchema));
});
