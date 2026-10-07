/**
 * Work tracking's words, as the classic pages say them: status and filter labels, link kinds and
 * pull request states, event times, owners and history lines. The tolerant fields
 * (`WorkLinkKind`, `WorkPullRequestState`, `WorkHistoryKind`) may hold values this client doesn't
 * know yet, so every lookup has a fallback.
 */
import type { WorkFilter } from "../../gen/WorkFilter.ts";
import type { WorkHistoryEntry } from "../../gen/WorkHistoryEntry.ts";
import type { WorkLink } from "../../gen/WorkLink.ts";
import type { WorkOwnerSnapshot } from "../../gen/WorkOwnerSnapshot.ts";
import type { WorkStatus } from "../../gen/WorkStatus.ts";
import type { IconName } from "../../ui/icons/icon.tsx";

/** The statuses, in the order the status menu offers them. */
export const WORK_STATUSES: readonly WorkStatus[] = ["planned", "in_progress", "blocked", "done"];

export const WORK_STATUS_LABEL: Readonly<Record<WorkStatus, string>> = {
  planned: "Planned",
  in_progress: "In progress",
  blocked: "Blocked",
  done: "Done",
};

/** What a status this client doesn't know says (statuses are read tolerantly). */
export const UNKNOWN_STATUS = "Unknown status";

/** Whether `status` is one of the four this client can show and set. */
export function isKnownStatus(status: string): status is WorkStatus {
  return WORK_STATUSES.some((known) => known === status);
}

/**
 * A status's label. The generated type lists four, but statuses decode tolerantly, so one added
 * later arrives as `"unknown"` and reads "Unknown status".
 */
export function workStatusLabel(status: string): string {
  return isKnownStatus(status) ? WORK_STATUS_LABEL[status] : UNKNOWN_STATUS;
}

/** A history snapshot's status: `null` is a thread that wasn't tracked. */
export function historyStatusLabel(status: string | null): string {
  return status === null ? "Ordinary thread" : workStatusLabel(status);
}

/** A null owner. */
export const UNASSIGNED = "Unassigned";

/** A history actor with no account (or none the client knows). */
export const FORMER_MEMBER = "Former member";

/** The work page's tabs. */
export const WORK_FILTER_LABEL: Readonly<Record<WorkFilter, string>> = {
  open: "Open work",
  done: "Completed",
  all: "All work",
  agents: "Owned by agents",
  boards: "Boards only",
};

/** What an empty tab says. */
export const WORK_FILTER_EMPTY: Readonly<Record<WorkFilter, string>> = {
  open: "No open work yet. Track a channel thread as work and it will appear here.",
  done: "No completed work yet. Work shows up here once its status is set to Done.",
  all: "No work threads yet. Track a channel thread as work and it will appear here.",
  agents: "No agent-owned work yet. Assign a work thread to an agent and it will appear here.",
  boards: "No board work yet. Create a post in one of your boards and it will appear here.",
};

/** A link's kind icon; a kind this client doesn't know gets a plain link. */
export function linkIcon(kind: string): IconName {
  switch (kind) {
    case "pull_request":
      return "git-pull-request";
    case "event":
      return "calendar";
    case "drive_file":
      return "file-text";
    default:
      return "link";
  }
}

/** A link's kind, for screen readers; "Link" for one this client doesn't know. */
export function linkKindLabel(kind: string): string {
  switch (kind) {
    case "pull_request":
      return "Pull request";
    case "event":
      return "Event";
    case "drive_file":
      return "Drive file";
    default:
      return "Link";
  }
}

/** A pull request's state label, or `null` for none (or a state this client doesn't know). */
export function pullRequestStateLabel(state: string | null): string | null {
  switch (state) {
    case "open":
      return "Open";
    case "draft":
      return "Draft";
    case "merged":
      return "Merged";
    case "closed":
      return "Closed";
    default:
      return null;
  }
}

/**
 * When an event starts, in the time zone it's shown in: "Wed, Oct 7, 3:00 PM EDT". Falls back to
 * the viewer's zone when the zone is missing or unknown to the browser.
 */
export function eventTimeLabel(startsAt: string, timeZone: string | null): string {
  const options: Intl.DateTimeFormatOptions = {
    weekday: "short",
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
    timeZoneName: "short",
  };

  const date = new Date(startsAt);

  const zoned: Intl.DateTimeFormatOptions = timeZone === null ? options : { ...options, timeZone };

  try {
    return new Intl.DateTimeFormat(undefined, zoned).format(date);
  } catch {
    return new Intl.DateTimeFormat(undefined, options).format(date);
  }
}

/** What a link says after its label: the PR state, or the event's time or cancellation. */
export function linkDetail(link: WorkLink): string | null {
  if (link.kind === "pull_request") {
    return pullRequestStateLabel(link.pullRequestState);
  }

  if (link.kind === "event") {
    if (link.eventCancelled) {
      return "Cancelled";
    }

    return link.eventStartsAt === null
      ? null
      : eventTimeLabel(link.eventStartsAt, link.eventTimeZone);
  }

  return null;
}

/** A link's accessible name: kind, label, title and detail. */
export function linkAccessibleName(link: WorkLink): string {
  const detail = linkDetail(link);

  return [linkKindLabel(link.kind), link.label, link.title, detail]
    .filter((part) => part !== null && part !== "")
    .join(", ");
}

/** A snapshot's name; no snapshot, or one with no name recorded, reads "Unassigned". */
export function snapshotName(snapshot: WorkOwnerSnapshot | null): string {
  return snapshot?.name ?? UNASSIGNED;
}

const sameOwner = (a: WorkOwnerSnapshot | null, b: WorkOwnerSnapshot | null): boolean =>
  (a?.userId ?? null) === (b?.userId ?? null) && (a?.name ?? null) === (b?.name ?? null);

/**
 * One history line after the actor's name, as the classic page writes it:
 * "status Planned → In progress · owner Unassigned → Maya · Note: …", "handed off owner Maya →
 * Ember · {summary} (2 links, 1 open question)", "updated the result".
 */
export function historyText(entry: WorkHistoryEntry): string {
  const owners = `owner ${snapshotName(entry.fromOwner)} → ${snapshotName(entry.toOwner)}`;
  const note = entry.note === null || entry.note === "" ? null : `Note: ${entry.note}`;

  switch (entry.kind) {
    case "handoff": {
      const handoff = entry.handoff;

      if (handoff === null) {
        return `handed off ${owners}`;
      }

      const links = `${handoff.linkCount} ${handoff.linkCount === 1 ? "link" : "links"}`;

      const questions = `${handoff.questionCount} open ${
        handoff.questionCount === 1 ? "question" : "questions"
      }`;

      return `handed off ${owners} · ${handoff.summary} (${links}, ${questions})`;
    }

    case "result":
      return "updated the result";
    case "update":
    case "assignment": {
      const parts: string[] = [];

      if (entry.fromStatus !== entry.toStatus) {
        parts.push(
          `status ${historyStatusLabel(entry.fromStatus)} → ${historyStatusLabel(entry.toStatus)}`,
        );
      }

      if (!sameOwner(entry.fromOwner, entry.toOwner)) {
        parts.push(owners);
      }

      if (note !== null) {
        parts.push(note);
      }

      return parts.length === 0 ? "changed the work" : parts.join(" · ");
    }

    default:
      return note === null ? "changed the work" : `changed the work · ${note}`;
  }
}

/** The history entry's icon. */
export function historyIcon(kind: string): IconName {
  switch (kind) {
    case "handoff":
      return "send";
    case "assignment":
      return "user-plus";
    case "result":
      return "file-text";
    default:
      return "circle-dot";
  }
}
