import { Link, useParams, useRouter } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { actions } from "../../sync/runtime.ts";
import { PageNotFound } from "../shell/not-found.tsx";
import { PageLoading } from "../shell/page-loading.tsx";
import { handoffRefusal } from "./handoff-access.ts";
import { linksRefusal } from "./links-access.ts";

interface Blocked {
  readonly threadId: number;
  readonly roomId: number;
  readonly message: string;
}

/**
 * `/app/t/$threadId/handoff`, where the classic handoff page
 * (`/threads/:id/work/handoff/new`) lands: it names only the thread, so this learns the thread's
 * room and replaces itself with the room's thread pane and its handoff dialog. A thread the
 * viewer can't see is a 404. A thread that isn't tracked, that this viewer can't manage
 * (a board post included), or that no agent can take, stays here and says why: sending it on
 * to the dialog and straight back would loop.
 */
export function HandoffResolver() {
  const params = useParams({ strict: false });
  const threadId = params.threadId ?? 0;
  const router = useRouter();
  const [missing, setMissing] = useState<number | null>(null);
  const [blocked, setBlocked] = useState<Blocked | null>(null);

  useEffect(() => {
    let current = true;

    setMissing(null);
    setBlocked(null);
    void actions.threads.read(threadId).then(
      (detail) => {
        if (!current) {
          return;
        }

        const message = handoffRefusal({
          tracked: detail.thread.work !== null,
          canManage: detail.permissions.canManageWork,
          receiverCount: detail.work?.handoffReceivers.length ?? null,
        });

        if (message !== null) {
          setBlocked({ threadId, roomId: detail.thread.roomId, message });

          return;
        }

        void router.navigate({
          to: "/r/$roomId/t/$threadId/handoff",
          params: { roomId: detail.thread.roomId, threadId },
          replace: true,
        });
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

  if (missing === threadId) {
    return <PageNotFound />;
  }

  if (blocked !== null && blocked.threadId === threadId) {
    return (
      <section className="room room-error enter-fade" aria-label="Can't hand this off">
        <p className="text-title" role="alert">
          {blocked.message}
        </p>
        <Link to="/r/$roomId/t/$threadId" params={{ roomId: blocked.roomId, threadId }}>
          Back to the thread
        </Link>
      </section>
    );
  }

  return <PageLoading />;
}

/**
 * `/app/t/$threadId/links`, where the classic links page (`/threads/:id/work/links`) lands: it
 * names only the thread, so this learns the thread's room and replaces itself with the room's
 * thread pane and its link editor. A thread the viewer can't see is a 404. A thread that isn't
 * tracked stays here and says why: the pane would open `/links` with nothing to edit.
 */
export function LinksResolver() {
  const params = useParams({ strict: false });
  const threadId = params.threadId ?? 0;
  const router = useRouter();
  const [missing, setMissing] = useState<number | null>(null);
  const [blocked, setBlocked] = useState<Blocked | null>(null);

  useEffect(() => {
    let current = true;

    setMissing(null);
    setBlocked(null);
    void actions.threads.read(threadId).then(
      (detail) => {
        if (!current) {
          return;
        }

        const message = linksRefusal({ tracked: detail.thread.work !== null });

        if (message !== null) {
          setBlocked({ threadId, roomId: detail.thread.roomId, message });

          return;
        }

        void router.navigate({
          to: "/r/$roomId/t/$threadId/links",
          params: { roomId: detail.thread.roomId, threadId },
          replace: true,
        });
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

  if (missing === threadId) {
    return <PageNotFound />;
  }

  if (blocked !== null && blocked.threadId === threadId) {
    return (
      <section className="room room-error enter-fade" aria-label="Can't link to this work">
        <p className="text-title" role="alert">
          {blocked.message}
        </p>
        <Link to="/r/$roomId/t/$threadId" params={{ roomId: blocked.roomId, threadId }}>
          Back to the thread
        </Link>
      </section>
    );
  }

  return <PageLoading />;
}
