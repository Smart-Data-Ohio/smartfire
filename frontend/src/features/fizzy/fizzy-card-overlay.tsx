import { useNavigate, useParams } from "@tanstack/react-router";
import { lazy, Suspense, useEffect, useRef, useState } from "react";
import type { CreatedFizzyCard } from "../../gen/CreatedFizzyCard.ts";
import { store } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { toast } from "../../ui/toast-store.ts";
import { type FizzyMessageScope, scopeKey } from "./fizzy-card-model.ts";
import { useCloseOverlay } from "./overlay-history.ts";
import { useSourceFollower } from "./source-focus.ts";

const FizzyCardDialog = lazy(() => import("./fizzy-card-dialog.tsx"));

interface Shown {
  readonly scope: FizzyMessageScope;
  /** Bumps per opening, so each opening reads the form afresh. */
  readonly opening: number;
}

/** The source row, for focus after the dialog when the menu that opened it is gone. */
function sourceRow(scope: FizzyMessageScope): HTMLElement | null {
  return document.querySelector<HTMLElement>(`[data-message-id="${scope.messageId}"]`);
}

/**
 * "Create Fizzy card" over the conversation, open while the URL is one of its routes:
 * `/app/r/$roomId/m/$sourceId/fizzy/new` for a message of the room's timeline and
 * `/app/r/$roomId/t/$threadId/m/$sourceId/fizzy/new` for a thread reply (the thread stays open
 * beneath). It keeps the last source while its exit plays.
 */
export function FizzyCardOverlay({ roomId }: { readonly roomId: number }) {
  const params = useParams({ strict: false });
  const navigate = useNavigate();
  const sourceId = params.sourceId;
  const threadId = params.threadId ?? null;

  const scope: FizzyMessageScope | null =
    sourceId === undefined ? null : { roomId, threadId, messageId: sourceId };

  const open = scope !== null;
  const [shown, setShown] = useState<Shown | null>(null);
  const [wasOpen, setWasOpen] = useState(false);

  if (
    open !== wasOpen ||
    (scope !== null && shown !== null && scopeKey(scope) !== scopeKey(shown.scope))
  ) {
    setWasOpen(open);

    if (scope !== null) {
      setShown({ scope, opening: (shown?.opening ?? 0) + 1 });
    }
  }

  // The opening a create's completion belongs to: closing, another opening and unmounting all end
  // it, so a create that completes afterwards (say Back left the form) neither closes a newer
  // opening nor steps back or navigates.
  const active = useRef<number | null>(null);
  const opening = open ? (shown?.opening ?? null) : null;

  useEffect(() => {
    active.current = opening;

    return () => {
      active.current = null;
    };
  }, [opening]);

  // Focus after closing follows the source row for a moment; a new opening, or unmounting, ends it.
  const sourceFocus = useSourceFollower(opening);

  // Someone opened the dialog's URL: closing lands on the source message in its conversation.
  const closeOverlay = useCloseOverlay(() => {
    if (shown === null) {
      return;
    }

    const { scope: source } = shown;

    if (source.threadId === null) {
      void navigate({
        to: "/r/$roomId/m/$messageId",
        params: { roomId: source.roomId, messageId: source.messageId },
        replace: true,
      });
    } else {
      void navigate({
        to: "/r/$roomId/t/$threadId",
        params: { roomId: source.roomId, threadId: source.threadId },
        search: { m: source.messageId },
        replace: true,
      });

      // The thread beneath opened on its newest replies, and it reads `?m=` only when it opens:
      // an older source is read in around it here.
      const loaded = store.getState().threadTimelines[source.threadId]?.ids ?? [];

      if (!loaded.includes(source.messageId)) {
        void actions.threads.reload(source.threadId, source.messageId).catch(() => undefined);
      }
    }
  });

  const close = () => {
    active.current = null;
    closeOverlay();
  };

  const submittedIn = shown?.opening ?? null;

  const created = (card: CreatedFizzyCard) => {
    toast({ title: card.notice, tone: "success" });

    if (submittedIn !== null && active.current === submittedIn) close();
  };

  if (shown === null) {
    return null;
  }

  return (
    <Suspense fallback={null}>
      <FizzyCardDialog
        key={shown.opening}
        scope={shown.scope}
        open={open}
        onClose={close}
        onCreated={created}
        returnFocus={() => sourceFocus.follow(() => sourceRow(shown.scope))}
        returnFocusFirst
      />
    </Suspense>
  );
}
