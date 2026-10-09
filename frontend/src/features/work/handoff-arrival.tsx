import { useMatchRoute } from "@tanstack/react-router";
import { useEffect, useRef } from "react";
import { useStore } from "../../store/store.ts";
import { toast } from "../../ui/toast-store.ts";
import { handoffRefusal } from "./handoff-access.ts";
import { useHandoffRoute } from "./handoff-dialog.tsx";

/**
 * The handoff URL over a thread that can't be handed off. The work bar and a board post only
 * mount the dialog for a manager of tracked work, so an untracked thread or a non-manager (a
 * board post's included) would otherwise sit on `/handoff` with nothing to see. Says why, once,
 * and leaves the URL the same way a close does: back when the app pushed it, otherwise replacing
 * it with the thread, so it never bounces onto itself.
 */
export function HandoffArrival({ threadId }: { readonly threadId: number }) {
  const matchRoute = useMatchRoute();
  const open = matchRoute({ to: "/r/$roomId/t/$threadId/handoff", includeSearch: false }) !== false;
  const tracked = useStore((state) => (state.threads[threadId]?.work ?? null) !== null);
  const ready = useStore((state) => state.threadPanes[threadId]?.status === "ready");

  const canManage = useStore(
    (state) => state.threadPanes[threadId]?.permissions?.canManageWork ?? null,
  );

  const { closeHandoff } = useHandoffRoute(threadId);
  const closeRef = useRef(closeHandoff);
  const refused = useRef(false);

  closeRef.current = closeHandoff;

  useEffect(() => {
    if (!open) {
      refused.current = false;

      return;
    }

    if (!ready || canManage === null || refused.current) {
      return;
    }

    const message = handoffRefusal({ tracked, canManage });

    if (message === null) {
      return;
    }

    refused.current = true;
    toast({ title: message, tone: "danger" });
    closeRef.current();
  }, [open, ready, canManage, tracked]);

  return null;
}
