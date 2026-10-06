import { useNavigate } from "@tanstack/react-router";
import { type KeyboardEvent, type MouseEvent, useState } from "react";
import type { ActivityAction } from "../../gen/ActivityAction.ts";
import type { ActivityItem } from "../../gen/ActivityItem.ts";
import type { ActivityState } from "../../gen/ActivityState.ts";
import type { ActivityTab } from "../../gen/ActivityTab.ts";
import { useActivityList, useActivityUnread } from "../../store/inbox-hooks.ts";
import { actions } from "../../sync/runtime.ts";
import type { IconName } from "../../ui/icons/icon.tsx";
import { Tabs } from "../../ui/tabs.tsx";
import { toast } from "../../ui/toast-store.ts";
import { useListMotion } from "../destinations/list-motion.ts";
import { PageFrame } from "../destinations/page-frame.tsx";
import { PagedList } from "../destinations/paged-list.tsx";
import { PointMenu, type PointMenuRequest, requestMenu } from "../destinations/point-menu.tsx";
import { PaneEmpty } from "../panes/pane-states.tsx";
import { useNow } from "../threads/use-now.ts";
import {
  ACTIVITY_TABS,
  type ActivityTarget,
  activityTarget,
  emptyCopy,
} from "./activity-format.ts";
import { ActivityMenuItems, ActivityRow } from "./activity-row.tsx";
import "./activity.css";

/** The inbox's state filter, in the header. */
const STATUS_ITEMS = [
  { value: "unread", label: "Unread" },
  { value: "read", label: "Read" },
  { value: "handled", label: "Handled" },
] as const satisfies readonly { value: ActivityState; label: string }[];

function isStatus(value: string): value is ActivityState {
  return STATUS_ITEMS.some((item) => item.value === value);
}

/** An empty list's glyph: caught up, nothing handled, or nothing at all. */
const EMPTY_ICON = {
  unread: "check-check",
  read: "inbox",
  handled: "circle-check",
} as const satisfies Record<ActivityState, IconName>;

const ACTION_FAILED = {
  read: "Couldn't mark it as read",
  unread: "Couldn't mark it as unread",
  handled: "Couldn't mark it as handled",
  unhandled: "Couldn't mark it as not handled",
} as const satisfies Record<ActivityAction, string>;

/** Follows an item's target: an SPA screen, or the classic page for one the SPA hasn't ported. */
function useFollowTarget(): (target: ActivityTarget) => void {
  const navigate = useNavigate();

  return (target) => {
    switch (target.kind) {
      case "room":
        void navigate({ to: "/r/$roomId", params: { roomId: target.roomId } });

        return;
      case "message":
        void navigate({
          to: "/r/$roomId/m/$messageId",
          params: { roomId: target.roomId, messageId: target.messageId },
        });

        return;
      case "thread":
        void navigate({
          to: "/r/$roomId/t/$threadId",
          params: { roomId: target.roomId, threadId: target.threadId },
          search: target.messageId === null ? {} : { m: target.messageId },
        });

        return;
      case "scheduled":
        void navigate({ to: "/scheduled" });

        return;
      case "saved":
        void navigate({ to: "/saved" });

        return;
      case "classic":
        window.location.assign(target.href);

        return;
      case "none":
        return;
    }
  };
}

interface ActivityPageProps {
  readonly tab: ActivityTab;
  readonly status: ActivityState;
  readonly onFilterChange: (tab: ActivityTab, status: ActivityState) => void;
}

/**
 * `/app/activity`: the inbox. Type tabs (the sliding tabs recipe) and an Unread / Read / Handled
 * switch; rows that open their source (marking them read), with read and handled toggles on hover,
 * in a context menu and on keys. Live items slide in at the top; one that stops matching the
 * filter (read or handled under Unread) folds away. The next page loads as the end nears.
 */
export function ActivityPage({ tab, status, onFilterChange }: ActivityPageProps) {
  const now = useNow();
  const view = useActivityList(tab, status);
  const unread = useActivityUnread();
  const follow = useFollowTarget();
  const [celebrated, setCelebrated] = useState<ReadonlySet<number>>(() => new Set());
  const [menu, setMenu] = useState<{ request: PointMenuRequest; item: ActivityItem } | null>(null);

  const rows = useListMotion(
    view.status === "ready" ? view.items : null,
    (item) => item.id,
    `${tab}:${status}`,
  );

  const open = (item: ActivityItem) => {
    actions.activity.open(item.id).catch(() => {
      // Opening still goes there; the read mark just didn't stick.
    });
    follow(activityTarget(item));
  };

  const change = (item: ActivityItem, action: ActivityAction) => {
    if (action === "handled") {
      setCelebrated((ids) => new Set([...ids, item.id]));
    }

    actions.activity.setState(item.id, action).catch((error: Error) => {
      toast({ title: ACTION_FAILED[action], description: error.message, tone: "danger" });
    });
  };

  const openMenu = (
    item: ActivityItem,
    event: MouseEvent<HTMLElement> | KeyboardEvent<HTMLElement>,
  ) => {
    const id = (menu?.request.id ?? 0) + 1;

    requestMenu(id, event, (request) => setMenu({ request, item }));
  };

  const copy = emptyCopy(tab, status);

  return (
    <PageFrame
      title="Activity"
      icon="inbox"
      meta={
        unread !== null && unread > 0 ? (
          <span className="page-count">
            <span aria-hidden="true">{unread > 99 ? "99+" : unread}</span>
            <span className="visually-hidden">{unread} unread</span>
          </span>
        ) : null
      }
      tools={
        <div className="activity-status">
          <Tabs
            label="Show"
            items={STATUS_ITEMS}
            value={status}
            onValueChange={(value) => {
              if (isStatus(value)) {
                onFilterChange(tab, value);
              }
            }}
          />
        </div>
      }
      toolbar={
        <Tabs
          label="Activity type"
          items={ACTIVITY_TABS}
          value={tab}
          onValueChange={(value) => {
            const next = ACTIVITY_TABS.find((item) => item.value === value);

            if (next !== undefined) {
              onFilterChange(next.value, status);
            }
          }}
        />
      }
    >
      <PagedList
        state={view}
        label={`${ACTIVITY_TABS.find((item) => item.value === tab)?.label ?? "All"} activity`}
        errorText="Your activity couldn't be loaded."
        isEmpty={rows.length === 0}
        empty={<PaneEmpty icon={EMPTY_ICON[status]} title={copy.title} text={copy.text} />}
      >
        {rows.map((row) => (
          <ActivityRow
            key={row.key}
            item={row.value}
            now={now}
            motion={row.motion}
            celebrate={celebrated.has(row.key)}
            onOpen={open}
            onAction={change}
            onMenu={openMenu}
          />
        ))}
      </PagedList>
      {menu === null ? null : (
        <PointMenu
          key={menu.request.id}
          request={menu.request}
          label="Activity actions"
          onClosed={(id) => setMenu((current) => (current?.request.id === id ? null : current))}
        >
          <ActivityMenuItems item={menu.item} onOpen={open} onAction={change} />
        </PointMenu>
      )}
    </PageFrame>
  );
}
