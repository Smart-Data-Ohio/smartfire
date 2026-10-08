/** The S4 work endpoints: the work list, a thread's work fields and the handoff to an agent. */
import { Effect } from "effect";
import type { CreateWorkHandoff } from "../gen/CreateWorkHandoff.ts";
import type { ThreadDetail } from "../gen/ThreadDetail.ts";
import type { UpdateWork } from "../gen/UpdateWork.ts";
import type { WorkFilter } from "../gen/WorkFilter.ts";
import type { WorkList } from "../gen/WorkList.ts";
import { call, get } from "./call.ts";
import { ThreadDetail as ThreadDetailSchema } from "./schema/thread.ts";
import { WorkList as WorkListSchema } from "./schema/work.ts";
import { wire } from "./wire.ts";

/** `GET /work?state=`: tracked threads in the viewer's rooms, most recently updated first. */
export const workList = Effect.fn("api.workList")(function* (state: WorkFilter) {
  return yield* call(get("/work", { state }), wire<WorkList>(WorkListSchema));
});

/** `PATCH /threads/:id/work`: track, move, assign or stop tracking, or edit the result. */
export const updateWork = Effect.fn("api.updateWork")(function* (
  threadId: number,
  body: UpdateWork,
) {
  return yield* call(
    { method: "PATCH", path: `/threads/${threadId}/work`, body },
    wire<ThreadDetail>(ThreadDetailSchema),
  );
});

/** `POST /threads/:id/work/handoff`: hands the work to an agent with a context package. */
export const handOffWork = Effect.fn("api.handOffWork")(function* (
  threadId: number,
  body: CreateWorkHandoff,
) {
  return yield* call(
    { method: "POST", path: `/threads/${threadId}/work/handoff`, body },
    wire<ThreadDetail>(ThreadDetailSchema),
  );
});
