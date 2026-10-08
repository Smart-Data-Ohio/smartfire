import { useParams, useRouter } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { actions } from "../../sync/runtime.ts";
import { PageNotFound } from "../shell/not-found.tsx";
import { PageLoading } from "../shell/page-loading.tsx";

/**
 * `/app/t/$threadId/handoff`, where the classic handoff page's URL (`/threads/:id/work/handoff/new`)
 * lands: it names only the thread, so this learns the thread's room and replaces itself with the
 * room's thread pane and its handoff dialog. A thread the viewer can't see is a 404.
 */
export function HandoffResolver() {
  const params = useParams({ strict: false });
  const threadId = params.threadId ?? 0;
  const router = useRouter();
  const [missing, setMissing] = useState<number | null>(null);

  useEffect(() => {
    let current = true;

    void actions.threads.locate(threadId).then(
      (roomId) => {
        if (current) {
          void router.navigate({
            to: "/r/$roomId/t/$threadId/handoff",
            params: { roomId, threadId },
            replace: true,
          });
        }
      },
      () => {
        if (current) {
          setMissing(threadId);
        }
      },
    );

    return () => {
      current = false;
    };
  }, [threadId, router]);

  return missing === threadId ? <PageNotFound /> : <PageLoading />;
}
