/**
 * Words, glyphs and destinations for the activity inbox: what each event type is called (the
 * classic inbox's labels), which tab holds it, what an empty tab says, and where opening an item
 * leads in the SPA (or, for a screen the SPA hasn't ported yet, on the classic site).
 */
import type { ActivityEventType } from "../../gen/ActivityEventType.ts";
import type { ActivityItem } from "../../gen/ActivityItem.ts";
import type { ActivityState } from "../../gen/ActivityState.ts";
import type { ActivityTab } from "../../gen/ActivityTab.ts";
import type { AgentApprovalStatus } from "../../gen/AgentApprovalStatus.ts";
import type { AgentBudgetCap } from "../../gen/AgentBudgetCap.ts";
import { activityDestination } from "../../store/activity.ts";
import type { IconName } from "../../ui/icons/icon.tsx";

/** The classic inbox's label for each event type (`presenters/activity.rs`). */
export const EVENT_LABEL = {
  mention: "Mention",
  reply: "Reply",
  thread_activity: "Followed thread",
  keyword_alert: "Keyword alert",
  work_update: "Work update",
  work_assignment: "Work assignment",
  work_sla: "SLA breach",
  huddle_started: "Incoming huddle",
  huddle_missed: "Missed huddle",
  event_invitation: "Event invitation",
  event_update: "Event update",
  event_cancelled: "Event cancelled",
  event_reminder: "Event reminder",
  pr_review_request: "Review requested",
  agent_approval_request: "Approval request",
  agent_budget_exceeded: "Budget exceeded",
  message_reminder: "Reminder",
  scheduled_message_dropped: "Scheduled message not sent",
  two_factor_lockout: "Sign-in lockout",
  new_sign_in: "New sign-in",
} as const satisfies Record<ActivityEventType, string>;

/** The glyph a row's kind badge (or, with no person behind it, its tile) shows. */
export const EVENT_ICON = {
  mention: "at",
  reply: "reply",
  thread_activity: "thread",
  keyword_alert: "bell",
  work_update: "briefcase",
  work_assignment: "briefcase",
  work_sla: "alarm-clock",
  huddle_started: "headphones",
  huddle_missed: "phone",
  event_invitation: "calendar",
  event_update: "calendar",
  event_cancelled: "calendar",
  event_reminder: "calendar-clock",
  pr_review_request: "git-pull-request",
  agent_approval_request: "bot",
  agent_budget_exceeded: "gauge",
  message_reminder: "alarm-clock",
  scheduled_message_dropped: "ban",
  two_factor_lockout: "shield",
  new_sign_in: "log-in",
} as const satisfies Record<ActivityEventType, IconName>;

/** A kind's colour family: amber for you, violet for agents, red for trouble. */
export type EventTone = "mention" | "agent" | "danger" | "accent" | "neutral";

export const EVENT_TONE = {
  mention: "mention",
  reply: "accent",
  thread_activity: "neutral",
  keyword_alert: "mention",
  work_update: "neutral",
  work_assignment: "accent",
  work_sla: "danger",
  huddle_started: "accent",
  huddle_missed: "neutral",
  event_invitation: "accent",
  event_update: "neutral",
  event_cancelled: "danger",
  event_reminder: "accent",
  pr_review_request: "accent",
  agent_approval_request: "agent",
  agent_budget_exceeded: "agent",
  message_reminder: "accent",
  scheduled_message_dropped: "danger",
  two_factor_lockout: "danger",
  new_sign_in: "neutral",
} as const satisfies Record<ActivityEventType, EventTone>;

export interface ActivityTabItem {
  readonly value: ActivityTab;
  readonly label: string;
}

/** The inbox's type tabs, in the classic page's order. */
export const ACTIVITY_TABS: readonly ActivityTabItem[] = [
  { value: "all", label: "All" },
  { value: "mentions", label: "Mentions" },
  { value: "threads", label: "Threads" },
  { value: "events", label: "Events" },
  { value: "agents", label: "Agents" },
  { value: "github", label: "GitHub" },
  { value: "huddles", label: "Huddles" },
  { value: "reminders", label: "Reminders" },
  { value: "security", label: "Security" },
];

/** What each tab collects, for its empty state ("No unread mentions or replies"). */
const TAB_NOUN = {
  all: "activity",
  mentions: "mentions or replies",
  threads: "thread or work updates",
  events: "event updates",
  agents: "agent requests",
  github: "review requests",
  huddles: "huddles",
  reminders: "reminders",
  security: "security notices",
} as const satisfies Record<ActivityTab, string>;

/** Where a tab's items come from, in one sentence, for its empty state. */
const TAB_HINT = {
  all: "Mentions, replies, followed threads, reminders and requests land here.",
  mentions: "When someone @-mentions you, replies to you or trips a keyword, it shows up here.",
  threads: "Replies in threads you follow and changes to your work show up here.",
  events: "Invitations, changes and reminders for your events show up here.",
  agents: "Approval requests and budget notices from your agents show up here.",
  github: "Pull request reviews requested of you show up here.",
  huddles: "Huddles you're called into, and ones you miss, show up here.",
  reminders: "Saved messages you asked to be reminded about show up here when they're due.",
  security: "New sign-ins and two-step lockouts on your account show up here.",
} as const satisfies Record<ActivityTab, string>;

export interface EmptyCopy {
  readonly title: string;
  readonly text: string;
}

/** What the Handled list holds, for its empty state. */
const HANDLED_HINT = "Mark an item handled once you've dealt with it, and it moves here.";

/**
 * An empty list's title and hint: caught up under Unread, nothing handled under Handled, and
 * nothing yet under Read.
 */
export function emptyCopy(tab: ActivityTab, status: ActivityState): EmptyCopy {
  switch (status) {
    case "unread":
      return {
        title: tab === "all" ? "You're all caught up" : `No unread ${TAB_NOUN[tab]}`,
        text: TAB_HINT[tab],
      };
    case "handled":
      return {
        title: tab === "all" ? "Nothing handled yet" : `No handled ${TAB_NOUN[tab]}`,
        text: HANDLED_HINT,
      };
    case "read":
      return {
        title: tab === "all" ? "No activity yet" : `No ${TAB_NOUN[tab]} yet`,
        text: TAB_HINT[tab],
      };
  }
}

/** Agent approval statuses as a chip reads them. */
export const APPROVAL_STATUS_LABEL = {
  pending: "Waiting for you",
  approved: "Approved",
  denied: "Denied",
  cancelled: "Cancelled",
  expired: "Expired",
} as const satisfies Record<AgentApprovalStatus, string>;

/** Which agent budget ran out, as a chip reads it. */
export const BUDGET_CAP_LABEL = {
  messages: "Message cap",
  board_posts: "Board post cap",
  external_actions: "External action cap",
} as const satisfies Record<AgentBudgetCap, string>;

/** An item's status chip: an approval's state or the budget that ran out; `null` for others. */
export function statusChip(
  item: ActivityItem,
): { readonly status: AgentApprovalStatus | "budget"; readonly label: string } | null {
  const { approvalStatus, budgetCap } = item.source;

  if (approvalStatus !== null) {
    return { status: approvalStatus, label: APPROVAL_STATUS_LABEL[approvalStatus] };
  }

  return budgetCap === null ? null : { status: "budget", label: BUDGET_CAP_LABEL[budgetCap] };
}

/** Where opening an item goes. */
export type ActivityTarget =
  | { readonly kind: "room"; readonly roomId: number }
  | { readonly kind: "message"; readonly roomId: number; readonly messageId: number }
  | {
      readonly kind: "thread";
      readonly roomId: number;
      readonly threadId: number;
      readonly messageId: number | null;
    }
  | { readonly kind: "scheduled" }
  | { readonly kind: "saved" }
  | { readonly kind: "classic"; readonly href: string }
  | { readonly kind: "none" };

function positiveId(text: string | null | undefined): number | null {
  if (text === null || text === undefined || !/^\d+$/.test(text)) {
    return null;
  }

  const id = Number(text);

  return Number.isSafeInteger(id) && id > 0 ? id : null;
}

/**
 * The SPA screen for a classic path, when there is one: `/rooms/12/@9001` (a message),
 * `/rooms/12?message_id=9&thread=5` (a reply), `/rooms/12?thread=5` (a thread), `/rooms/12`,
 * `/scheduled_messages` and `/saved`. Anything else (events, approvals, sessions, settings)
 * stays on the classic site.
 */
export function targetFromPath(path: string): ActivityTarget {
  if (path === "") {
    return { kind: "none" };
  }

  const url = new URL(path, "https://smartfire.invalid");
  const pathname = url.pathname.replace(/\/+$/, "");

  if (pathname === "/scheduled_messages") {
    return { kind: "scheduled" };
  }

  if (pathname === "/saved") {
    return { kind: "saved" };
  }

  const room = /^\/rooms\/(\d+)(?:\/@(\d+))?$/.exec(pathname);
  const roomId = positiveId(room?.[1]);

  if (room === null || roomId === null) {
    return { kind: "classic", href: path };
  }

  const atMessage = positiveId(room[2]);
  const threadId = positiveId(url.searchParams.get("thread"));
  const messageId = atMessage ?? positiveId(url.searchParams.get("message_id"));

  if (threadId !== null) {
    return { kind: "thread", roomId, threadId, messageId };
  }

  return messageId === null ? { kind: "room", roomId } : { kind: "message", roomId, messageId };
}

/**
 * Where opening `item` leads: the data layer's destination from the item's ids, with a classic
 * path mapped into the SPA where it has the screen (`targetFromPath`).
 */
export function activityTarget(item: ActivityItem): ActivityTarget {
  const destination = activityDestination(item);

  switch (destination.kind) {
    case "message":
      return destination.threadId === null
        ? { kind: "message", roomId: destination.roomId, messageId: destination.messageId }
        : {
            kind: "thread",
            roomId: destination.roomId,
            threadId: destination.threadId,
            messageId: destination.messageId,
          };
    case "thread":
      return { ...destination, messageId: null };
    case "classic":
      return targetFromPath(destination.path);
    case "room":
    case "scheduled":
      return destination;
  }
}
