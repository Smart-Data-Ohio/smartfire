import { Effect } from "effect";
import type { CreateWorkLink } from "../gen/CreateWorkLink.ts";
import type { ThreadDetail } from "../gen/ThreadDetail.ts";
import type { WorkLinkForm } from "../gen/WorkLinkForm.ts";
import { call, get } from "./call.ts";
import { ThreadDetail as ThreadDetailSchema } from "./schema/thread.ts";
import { WorkLinkForm as WorkLinkFormSchema } from "./schema/work-links.ts";
import { wire } from "./wire.ts";

export const workLinkForm = Effect.fn("api.workLinkForm")(function* (threadId: number) {
  return yield* call(
    get(`/threads/${threadId}/work/links/new`),
    wire<WorkLinkForm>(WorkLinkFormSchema),
  );
});

export const createWorkLink = Effect.fn("api.createWorkLink")(function* (
  threadId: number,
  body: CreateWorkLink,
) {
  return yield* call(
    { method: "POST", path: `/threads/${threadId}/work/links`, body },
    wire<ThreadDetail>(ThreadDetailSchema),
  );
});

export const deleteWorkLink = Effect.fn("api.deleteWorkLink")(function* (
  threadId: number,
  linkId: number,
) {
  return yield* call(
    { method: "DELETE", path: `/threads/${threadId}/work/links/${linkId}` },
    wire<ThreadDetail>(ThreadDetailSchema),
  );
});
