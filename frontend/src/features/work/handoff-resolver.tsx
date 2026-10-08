import { useParams, useRouter } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { actions } from "../../sync/runtime.ts";
import { PageNotFound } from "../shell/not-found.tsx";
import { PageLoading } from "../shell/page-loading.tsx";

/** Where a resolved work page opens: over the room's thread pane. */
type Destination = "/r/$roomId/t/$threadId/handoff" | "/r/$roomId/t/$threadId/links";

/**
 * A classic work page's URL names only the thread, so this learns the thread's room and replaces
 * itself with `to` in that room. A thread the viewer can't see is a 404.
 */
function ThreadPageResolver({ to }: { readonly to: Destination }) {
  const params = useParams({ strict: false });
  const threadId = params.threadId ?? 0;
  const router = useRouter();
  const [missing, setMissing] = useState<number | null>(null);

  useEffect(() => {
    let current = true;

    void actions.threads.locate(threadId).then(
      (roomId) => {
        if (current) {
          void router.navigate({ to, params: { roomId, threadId }, replace: true });
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
  }, [threadId, router, to]);

  return missing === threadId ? <PageNotFound /> : <PageLoading />;
}

/** `/app/t/$threadId/handoff`, where the classic handoff page (`/threads/:id/work/handoff/new`) lands. */
export function HandoffResolver() {
  return <ThreadPageResolver to="/r/$roomId/t/$threadId/handoff" />;
}

/** `/app/t/$threadId/links`, where the classic links page (`/threads/:id/work/links`) lands. */
export function LinksResolver() {
  return <ThreadPageResolver to="/r/$roomId/t/$threadId/links" />;
}
