import { useNavigate } from "@tanstack/react-router";
import { type KeyboardEvent, type MouseEvent, useState } from "react";
import type { SavedFilter } from "../../gen/SavedFilter.ts";
import type { SavedItem } from "../../gen/SavedItem.ts";
import { type SavedRow as SavedEntry, useSavedList } from "../../store/inbox-hooks.ts";
import { actions } from "../../sync/runtime.ts";
import { Tabs } from "../../ui/tabs.tsx";
import { toast } from "../../ui/toast-store.ts";
import { LazyCustomTimeDialog } from "../composer/schedule/lazy-custom-time-dialog.tsx";
import { sendAtLabel } from "../composer/schedule/presets.ts";
import { useListMotion } from "../destinations/list-motion.ts";
import { PageFrame } from "../destinations/page-frame.tsx";
import { PagedList } from "../destinations/paged-list.tsx";
import { PointMenu, type PointMenuRequest, requestMenu } from "../destinations/point-menu.tsx";
import { PaneEmpty } from "../panes/pane-states.tsx";
import { useNow } from "../threads/use-now.ts";
import { SavedMenuItems, SavedRow, type SavedRowHandlers } from "./saved-row.tsx";
import "./saved.css";

/** The page's filter tabs. */
export const SAVED_TABS = [
  { value: "in_progress", label: "In progress" },
  { value: "done", label: "Done" },
  { value: "all", label: "All" },
] as const satisfies readonly { value: SavedFilter; label: string }[];

export function isSavedFilter(value: string): value is SavedFilter {
  return SAVED_TABS.some((tab) => tab.value === value);
}

const EMPTY = {
  in_progress: {
    title: "Nothing saved for later",
    text: "Save a message from its menu (or press S on it) to keep it here until you're done with it.",
  },
  done: {
    title: "Nothing done yet",
    text: "Mark a saved message done when you've dealt with it, and it moves here.",
  },
  all: {
    title: "No saved messages",
    text: "Save a message from its menu (or press S on it) to come back to it later.",
  },
} as const satisfies Record<SavedFilter, { title: string; text: string }>;

function failed(title: string) {
  return (error: Error) => toast({ title, description: error.message, tone: "danger" });
}

/** "Reminder set for tomorrow at 9:00 AM". */
function reminderToast(at: Date): string {
  return `Reminder set for ${sendAtLabel(at, new Date()).replace(/^(Today|Tomorrow)/, (word) => word.toLowerCase())}`;
}

interface SavedPageProps {
  readonly filter: SavedFilter;
  readonly onFilterChange: (filter: SavedFilter) => void;
}

/**
 * `/app/saved`: Slack's "Later". In progress / Done / All; each row shows the message with its
 * conversation, attachment and reminder. Open jumps to it; done, remind and remove sit on hover,
 * in the menu and on keys. Removing offers Undo.
 */
export function SavedPage({ filter, onFilterChange }: SavedPageProps) {
  const now = useNow();
  const navigate = useNavigate();
  const view = useSavedList(filter);
  const [celebrated, setCelebrated] = useState<ReadonlySet<number>>(() => new Set());
  const [custom, setCustom] = useState<SavedItem | null>(null);
  const [menu, setMenu] = useState<{ request: PointMenuRequest; item: SavedItem } | null>(null);

  const rows = useListMotion(
    view.status === "ready" ? view.rows : null,
    (row: SavedEntry) => row.item.id,
    filter,
  );

  const byItem = new Map(rows.map((row) => [row.key, row.value]));

  const remind = (item: SavedItem, at: Date | null) =>
    actions.saved.setReminder(item.messageId, at === null ? null : at.toISOString());

  const handlers: SavedRowHandlers = {
    onOpen: (item) => {
      const message = byItem.get(item.id)?.message ?? null;

      if (message === null) {
        return;
      }

      if (message.threadId === null) {
        void navigate({
          to: "/r/$roomId/m/$messageId",
          params: { roomId: message.roomId, messageId: message.id },
        });
      } else {
        void navigate({
          to: "/r/$roomId/t/$threadId",
          params: { roomId: message.roomId, threadId: message.threadId },
          search: { m: message.id },
        });
      }
    },
    onToggleDone: (item) => {
      const next = item.status === "done" ? "in_progress" : "done";

      if (next === "done") {
        setCelebrated((ids) => new Set([...ids, item.id]));
      }

      actions.saved
        .setStatus(item.id, next)
        .catch(failed(next === "done" ? "Couldn't mark it as done" : "Couldn't reopen it"));
    },
    onRemind: (item, at) => {
      remind(item, at).then(
        () => toast({ title: at === null ? "Reminder cleared" : reminderToast(at) }),
        failed(at === null ? "Couldn't clear the reminder" : "Couldn't set the reminder"),
      );
    },
    onCustomRemind: (item) => setCustom(item),
    onRemove: (item) => {
      actions.saved.remove(item.id).then(
        () =>
          toast({
            title: "Removed from saved",
            action: {
              label: "Undo",
              onClick: () => {
                actions.saved.restore(item).catch(failed("Couldn't restore it"));
              },
            },
          }),
        failed("Couldn't remove it"),
      );
    },
    onMenu: (item, event: MouseEvent<HTMLElement> | KeyboardEvent<HTMLElement>) => {
      const id = (menu?.request.id ?? 0) + 1;

      requestMenu(id, event, (request) => setMenu({ request, item }));
    },
  };

  const empty = EMPTY[filter];

  return (
    <PageFrame
      title="Saved"
      icon="bookmark"
      back
      toolbar={
        <Tabs
          label="Show"
          items={SAVED_TABS}
          value={filter}
          onValueChange={(value) => {
            if (isSavedFilter(value)) {
              onFilterChange(value);
            }
          }}
        />
      }
    >
      <PagedList
        state={view}
        label="Saved messages"
        errorText="Your saved messages couldn't be loaded."
        isEmpty={rows.length === 0}
        empty={<PaneEmpty icon="bookmark" title={empty.title} text={empty.text} />}
      >
        {rows.map((row) => (
          <SavedRow
            key={row.key}
            item={row.value.item}
            message={row.value.message}
            conversation={row.value.conversation}
            now={now}
            motion={row.motion}
            celebrate={celebrated.has(row.key)}
            handlers={handlers}
          />
        ))}
      </PagedList>
      {menu === null ? null : (
        <PointMenu
          key={menu.request.id}
          request={menu.request}
          label="Saved message actions"
          onClosed={(id) => setMenu((current) => (current?.request.id === id ? null : current))}
        >
          <SavedMenuItems item={menu.item} handlers={handlers} />
        </PointMenu>
      )}
      <LazyCustomTimeDialog
        open={custom !== null}
        onOpenChange={(open) => {
          if (!open) {
            setCustom(null);
          }
        }}
        title="Remind me about this message"
        confirmLabel="Set reminder"
        initial={custom?.remindAt == null ? null : new Date(custom.remindAt)}
        onConfirm={async (at) => {
          if (custom !== null) {
            await remind(custom, at);
            toast({ title: reminderToast(at) });
          }
        }}
      />
    </PageFrame>
  );
}
