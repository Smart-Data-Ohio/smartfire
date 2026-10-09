import { useNavigate } from "@tanstack/react-router";
import { type ReactElement, Suspense, useState } from "react";
import type { ScheduledMessage } from "../../gen/ScheduledMessage.ts";
import { lazyForUpdate as lazy } from "../../service-worker/lazy.ts";
import { type ScheduledRow as ScheduledEntry, useScheduledList } from "../../store/inbox-hooks.ts";
import { actions, isScheduledDropped } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { useOpenedOnce } from "../../ui/opened-once.ts";
import { toast } from "../../ui/toast-store.ts";
import { LazyCustomTimeDialog } from "../composer/schedule/lazy-custom-time-dialog.tsx";
import { conversationText } from "../destinations/conversation-label.tsx";
import { type MotionRow, useListMotion } from "../destinations/list-motion.ts";
import { PageFrame } from "../destinations/page-frame.tsx";
import { PagedList, type PagedState } from "../destinations/paged-list.tsx";
import { PointMenu, type PointMenuRequest, requestMenu } from "../destinations/point-menu.tsx";
import { PaneEmpty } from "../panes/pane-states.tsx";
import { useNow } from "../threads/use-now.ts";
import type { ScheduledEdit } from "./edit-scheduled-dialog.tsx";
import { inlineWhen, markdownExcerpt, scheduledSection } from "./scheduled-format.ts";
import { ScheduledMenuItems, ScheduledRow, type ScheduledRowHandlers } from "./scheduled-row.tsx";
import "./scheduled.css";

const LoadedEditDialog = lazy(async () => {
  const module = await import("./edit-scheduled-dialog.tsx");

  return { default: module.EditScheduledDialog };
});

/** The editor, loaded the first time a message is edited. */
function LazyEditDialog(props: {
  readonly item: ScheduledMessage | null;
  readonly onClose: () => void;
  readonly onSave: (item: ScheduledMessage, edit: ScheduledEdit) => Promise<void>;
}) {
  const opened = useOpenedOnce(props.item !== null);

  return opened ? (
    <Suspense fallback={null}>
      <LoadedEditDialog {...props} />
    </Suspense>
  ) : null;
}

/**
 * Upcoming and stranded come from the pending list, past from its own: paging walks the pending
 * list to its end before Past shows (it sorts after every pending one), then pages Past.
 */
function combined(pending: PagedState, past: PagedState): PagedState {
  const onPast = pending.status === "ready" && !pending.hasMore;

  return {
    status: pending.status,
    loadingMore: pending.loadingMore || (onPast && past.loadingMore),
    hasMore: pending.hasMore || (onPast && (past.status !== "ready" || past.hasMore)),
    error: pending.error ?? (onPast ? past.error : null),
    loadMore: onPast ? (past.status === "error" ? past.reload : past.loadMore) : pending.loadMore,
    reload: () => {
      pending.reload();
      past.reload();
    },
  };
}

function Section({
  id,
  title,
  count,
  hint,
}: {
  id: string;
  title: string;
  count: number;
  hint?: string;
}) {
  return (
    // biome-ignore lint/a11y/useSemanticElements: virtua renders its items inside divs; a heading in a list needs an item of its own
    <div role="listitem">
      <h2 className="page-section-title" id={`scheduled-${id}`}>
        {title}
        <span className="page-section-count">{count}</span>
      </h2>
      {hint === undefined ? null : <p className="page-section-hint">{hint}</p>}
    </div>
  );
}

/**
 * `/app/scheduled`: every message the viewer scheduled, in the classic page's three groups.
 * Upcoming (send now, reschedule, edit, cancel), Stranded (pending ones that can't be posted any
 * more, with why), and Past (sent, with View message, or dropped, with the reason). Rows move
 * between the groups live as messages go out.
 */
export function ScheduledPage() {
  const now = useNow();
  const navigate = useNavigate();
  const pending = useScheduledList("pending");
  const past = useScheduledList("past");
  const [sending, setSending] = useState<ReadonlySet<number>>(() => new Set());
  const [editing, setEditing] = useState<ScheduledMessage | null>(null);
  const [rescheduling, setRescheduling] = useState<ScheduledMessage | null>(null);
  const [cancelling, setCancelling] = useState<ScheduledMessage | null>(null);
  const [menu, setMenu] = useState<{ request: PointMenuRequest; itemId: number } | null>(null);
  const keyOf = (row: ScheduledEntry) => row.message.id;

  const pendingRows = useListMotion(
    pending.status === "ready" ? pending.rows : null,
    keyOf,
    "pending",
  );

  const pastRows = useListMotion(past.status === "ready" ? past.rows : null, keyOf, "past");
  const state = combined(pending, past);
  const pastShown = pending.status === "ready" && !pending.hasMore;

  const settle = (id: number) =>
    setSending((ids) => new Set([...ids].filter((each) => each !== id)));

  const handlers: ScheduledRowHandlers = {
    onEdit: (item) => setEditing(item),
    onReschedule: (item) => setRescheduling(item),
    onCancel: (item) => setCancelling(item),
    onView: (item) => {
      if (item.sentMessageId === null) {
        return;
      }

      if (item.threadId === null) {
        void navigate({
          to: "/r/$roomId/m/$messageId",
          params: { roomId: item.roomId, messageId: item.sentMessageId },
        });
      } else {
        void navigate({
          to: "/r/$roomId/t/$threadId",
          params: { roomId: item.roomId, threadId: item.threadId },
          search: { m: item.sentMessageId },
        });
      }
    },
    onSendNow: (item) => {
      setSending((ids) => new Set([...ids, item.id]));
      actions.scheduled
        .sendNow(item.id)
        .then(
          (outcome) =>
            outcome === "sent"
              ? toast({ title: "Message sent", tone: "success" })
              : toast({
                  title: "Still scheduled",
                  description:
                    "It's already on its way, or its thread is locked. It stays scheduled and goes out as soon as it can.",
                }),
          (error: Error) =>
            toast(
              isScheduledDropped(error)
                ? { title: "Message dropped", description: error.message, tone: "danger" }
                : { title: "The message wasn't sent", description: error.message, tone: "danger" },
            ),
        )
        .finally(() => settle(item.id));
    },
    onMenu: (item, source) => {
      const id = (menu?.request.id ?? 0) + 1;

      requestMenu(id, source, (request) => setMenu({ request, itemId: item.id }));
    },
  };

  const byId = new Map([...pendingRows, ...pastRows].map((row) => [row.key, row.value]));

  const conversationOf = (item: ScheduledMessage | null) =>
    item === null ? null : (byId.get(item.id)?.conversation ?? null);

  // A message that just went out is briefly in both lists (leaving Upcoming, arriving in Past), so
  // each list keys its rows apart.
  const rowOf = (list: "pending" | "past") => (row: MotionRow<ScheduledEntry>) => (
    <ScheduledRow
      key={`${list}-${row.key}`}
      item={row.value.message}
      conversation={row.value.conversation}
      now={now}
      motion={row.motion}
      sending={sending.has(row.key)}
      handlers={handlers}
    />
  );

  const upcoming = pendingRows.filter((row) => scheduledSection(row.value.message) === "upcoming");
  const stranded = pendingRows.filter((row) => scheduledSection(row.value.message) === "stranded");
  const shownPast = pastShown ? pastRows : [];
  const children: ReactElement[] = [];

  if (upcoming.length > 0) {
    children.push(
      <Section key="upcoming" id="upcoming" title="Upcoming" count={upcoming.length} />,
    );
    children.push(...upcoming.map(rowOf("pending")));
  }

  if (stranded.length > 0) {
    children.push(
      <Section
        key="stranded"
        id="stranded"
        title="Can't be sent"
        count={stranded.length}
        hint="You left these conversations, or they were deleted. Each one is dropped when it's due unless you get access back."
      />,
    );
    children.push(...stranded.map(rowOf("pending")));
  }

  if (shownPast.length > 0) {
    children.push(<Section key="past" id="past" title="Past" count={shownPast.length} />);
    children.push(...shownPast.map(rowOf("past")));
  }

  const cancelConversation = conversationOf(cancelling);
  // The menu reads the row live, so it follows the message out of Upcoming while it's open.
  const menuItem = menu === null ? undefined : byId.get(menu.itemId)?.message;

  return (
    <PageFrame title="Scheduled" icon="clock" back>
      <PagedList
        state={state}
        label="Scheduled messages"
        errorText="Your scheduled messages couldn't be loaded."
        isEmpty={children.length === 0}
        empty={
          <PaneEmpty
            icon="clock"
            title="Nothing scheduled"
            text="Pick “Schedule for later” from the send button's menu to send a message at a better time. It shows up here until it goes out."
          />
        }
      >
        {children}
      </PagedList>
      {menu === null || menuItem === undefined ? null : (
        <PointMenu
          key={menu.request.id}
          request={menu.request}
          label="Scheduled message actions"
          onClosed={(id) => setMenu((current) => (current?.request.id === id ? null : current))}
        >
          <ScheduledMenuItems
            item={menuItem}
            sending={sending.has(menuItem.id)}
            handlers={handlers}
          />
        </PointMenu>
      )}
      <LazyEditDialog
        item={editing}
        onClose={() => setEditing(null)}
        onSave={async (item, edit) => {
          await actions.scheduled.update(item.id, edit);
          toast({ title: "Scheduled message updated", tone: "success" });
        }}
      />
      <LazyCustomTimeDialog
        open={rescheduling !== null}
        onOpenChange={(open) => {
          if (!open) {
            setRescheduling(null);
          }
        }}
        title="Reschedule message"
        confirmLabel="Reschedule"
        initial={rescheduling === null ? null : new Date(rescheduling.sendAt)}
        onConfirm={async (at) => {
          if (rescheduling !== null) {
            await actions.scheduled.update(rescheduling.id, { sendAt: at.toISOString() });
            toast({ title: `Rescheduled for ${inlineWhen(at.toISOString(), Date.now())}` });
          }
        }}
      />
      <Dialog
        open={cancelling !== null}
        onOpenChange={(open) => {
          if (!open) {
            setCancelling(null);
          }
        }}
        role="alertdialog"
        size="sm"
        title="Cancel this scheduled message?"
        description={`It won't be sent to ${conversationText(cancelConversation)}. This can't be undone.`}
        footer={
          <>
            <Button variant="secondary" onClick={() => setCancelling(null)} data-autofocus>
              Keep it
            </Button>
            <Button
              variant="danger"
              icon="trash"
              onClick={() => {
                const item = cancelling;

                setCancelling(null);

                if (item !== null) {
                  actions.scheduled.cancel(item.id).then(
                    () => toast({ title: "Scheduled message cancelled" }),
                    (error: Error) =>
                      toast({
                        title: "Couldn't cancel it",
                        description: error.message,
                        tone: "danger",
                      }),
                  );
                }
              }}
            >
              Cancel message
            </Button>
          </>
        }
      >
        {cancelling === null ? null : (
          <p className="scheduled-cancel-preview">{markdownExcerpt(cancelling.markdownSource)}</p>
        )}
      </Dialog>
    </PageFrame>
  );
}
