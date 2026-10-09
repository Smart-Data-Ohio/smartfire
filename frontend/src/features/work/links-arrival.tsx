import { useEffect, useRef } from "react";
import { useStore } from "../../store/store.ts";
import { toast } from "../../ui/toast-store.ts";
import { linksRefusal } from "./links-access.ts";
import { useLinksRoute } from "./links-route.ts";

/**
 * The links URL over a thread that isn't tracked. A board post and the work bar only mount the
 * editor for tracked work, so a pasted `/links` on an ordinary thread would otherwise sit there
 * with nothing to edit. Says why, once, and leaves the URL the same way a close does: back when
 * the app pushed it, otherwise replacing it with the thread.
 */
export function LinksArrival({ threadId }: { readonly threadId: number }) {
  const { open, closeLinks } = useLinksRoute(threadId);
  const located = useStore((state) => state.threads[threadId]?.roomId ?? null);
  const tracked = useStore((state) => (state.threads[threadId]?.work ?? null) !== null);
  const ready = useStore((state) => state.threadPanes[threadId]?.status === "ready");
  const closeRef = useRef(closeLinks);
  const refused = useRef(false);

  closeRef.current = closeLinks;

  useEffect(() => {
    if (!open) {
      refused.current = false;

      return;
    }

    // The pane's detail is what says whether this thread is work. Until it has landed, and
    // until the thread's room is known, there's nothing honest to say.
    if (!ready || located === null || refused.current) {
      return;
    }

    const message = linksRefusal({ tracked });

    if (message === null) {
      return;
    }

    refused.current = true;
    toast({ title: message, tone: "danger" });
    closeRef.current();
  }, [open, ready, located, tracked]);

  return null;
}
