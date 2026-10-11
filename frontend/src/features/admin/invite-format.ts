/** The invite manager's words and the small rules behind them. */
import type { WorkspaceInvite } from "../../gen/WorkspaceInvite.ts";
import type { WorkspaceInviteExpiry } from "../../gen/WorkspaceInviteExpiry.ts";
import type { WorkspaceInviteState } from "../../gen/WorkspaceInviteState.ts";
import type { Choice } from "../settings/settings-format.ts";

/** How long a new invite lasts: Discord's "Expire after" choices, which the server takes. */
export const EXPIRY_CHOICES: readonly Choice<WorkspaceInviteExpiry>[] = [
  { value: "30m", label: "30 minutes" },
  { value: "1h", label: "1 hour" },
  { value: "6h", label: "6 hours" },
  { value: "12h", label: "12 hours" },
  { value: "1d", label: "1 day" },
  { value: "7d", label: "7 days" },
  { value: "never", label: "Never" },
];

/** Discord's default: a week. */
export const DEFAULT_EXPIRY: WorkspaceInviteExpiry = "7d";

/** A use limit as the select holds it: `""` is no limit. */
export type UseLimit = "" | "1" | "5" | "10" | "25" | "50" | "100";

/** Discord's "Max number of uses" choices, the only limits the server accepts. */
export const USE_CHOICES: readonly Choice<UseLimit>[] = [
  { value: "", label: "No limit" },
  { value: "1", label: "1 use" },
  { value: "5", label: "5 uses" },
  { value: "10", label: "10 uses" },
  { value: "25", label: "25 uses" },
  { value: "50", label: "50 uses" },
  { value: "100", label: "100 uses" },
];

/** The wire's `maxUses` for a chosen limit. */
export function maxUsesOf(limit: UseLimit): number | null {
  return limit === "" ? null : Number(limit);
}

/** What the list can be narrowed to. */
export type InviteFilter = "all" | WorkspaceInviteState;

/** Each state's short name, as the badge and the filter say it. */
export const STATE_LABEL: Readonly<Record<WorkspaceInviteState, string>> = {
  active: "Active",
  expired: "Expired",
  exhausted: "Used up",
  revoked: "Revoked",
};

export const FILTERS: readonly { readonly value: InviteFilter; readonly label: string }[] = [
  { value: "all", label: "All" },
  { value: "active", label: STATE_LABEL.active },
  { value: "expired", label: STATE_LABEL.expired },
  { value: "exhausted", label: STATE_LABEL.exhausted },
  { value: "revoked", label: STATE_LABEL.revoked },
];

/**
 * Where an invite stands at `now`: the server's answer, except that an active invite whose time
 * ran out while the page was open is expired, as the server would now say.
 */
export function inviteState(invite: WorkspaceInvite, now: number): WorkspaceInviteState {
  if (
    invite.state === "active" &&
    invite.expiresAt !== null &&
    Date.parse(invite.expiresAt) <= now
  ) {
    return "expired";
  }

  return invite.state;
}

/** The invites the filter keeps, in the order given. */
export function filterInvites(
  invites: readonly WorkspaceInvite[],
  filter: InviteFilter,
  now: number,
): WorkspaceInvite[] {
  return invites.filter((invite) => filter === "all" || inviteState(invite, now) === filter);
}

/**
 * The list with invites made on the page that it doesn't have yet (it answered from before them)
 * first, newest first; an invite it has is taken as it says.
 */
export function withMade(
  made: readonly WorkspaceInvite[],
  listed: readonly WorkspaceInvite[],
): WorkspaceInvite[] {
  const known = new Set(listed.map((invite) => invite.id));

  return [...made.filter((invite) => !known.has(invite.id)), ...listed];
}

/**
 * A moment in the list, short enough for a table cell: "Oct 9, 3:00 PM", with the year only when
 * it isn't this one ("Dec 30, 2025, 3:00 PM").
 */
export function shortTime(timestamp: string, now: number, locale?: string): string {
  const date = new Date(timestamp);
  const thisYear = date.getFullYear() === new Date(now).getFullYear();

  return date.toLocaleString(locale, {
    year: thisYear ? undefined : "numeric",
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  });
}

/** "3 of 10", or "3, no limit". */
export function usesLabel(invite: WorkspaceInvite): string {
  return invite.maxUses === null
    ? `${invite.uses}, no limit`
    : `${invite.uses} of ${invite.maxUses}`;
}

const MINUTE_MS = 60_000;

const HOUR_MS = 60 * MINUTE_MS;

const DAY_MS = 24 * HOUR_MS;

/**
 * "in 25 minutes", "in 6 hours", "in 3 days": how long an invite has left, minutes rounded up and
 * hours and days to the nearest (so a day-long invite says "in 24 hours" for its first half hour).
 */
export function timeLeft(expiresAt: string, now: number, locale = "en"): string {
  const left = Math.max(0, Date.parse(expiresAt) - now);
  const relative = new Intl.RelativeTimeFormat(locale, { numeric: "always" });

  if (left < HOUR_MS) {
    return relative.format(Math.max(1, Math.ceil(left / MINUTE_MS)), "minute");
  }

  if (left < 2 * DAY_MS) {
    return relative.format(Math.max(1, Math.round(left / HOUR_MS)), "hour");
  }

  return relative.format(Math.round(left / DAY_MS), "day");
}

/**
 * The expiry column: "Never", how long a live invite has left ("in 6 hours"), or when a dead one
 * ran out (`at`, the moment in the viewer's locale).
 */
export function expiresLabel(
  invite: WorkspaceInvite,
  now: number,
  at: (timestamp: string) => string,
): string {
  if (invite.expiresAt === null) {
    return "Never";
  }

  return Date.parse(invite.expiresAt) > now
    ? timeLeft(invite.expiresAt, now)
    : at(invite.expiresAt);
}

/** What a new link was made for, under it: "Expires in 7 days. Works for 5 people." */
export function linkTerms(expiry: WorkspaceInviteExpiry, maxUses: number | null): string {
  const choice = EXPIRY_CHOICES.find((each) => each.value === expiry);

  const lasts =
    expiry === "never" || choice === undefined ? "Never expires." : `Expires in ${choice.label}.`;

  const uses =
    maxUses === null
      ? "No limit on how many people can join with it."
      : `Works for ${maxUses === 1 ? "1 person" : `${maxUses} people`}.`;

  return `${lasts} ${uses}`;
}

/** What the revoke confirmation says. */
export function revokeConfirmation(invite: WorkspaceInvite): string {
  const joined =
    invite.uses === 0
      ? "No one has joined with it yet."
      : `The ${invite.uses === 1 ? "person" : `${invite.uses} people`} who joined with it ${invite.uses === 1 ? "stays" : "stay"}.`;

  return `Anyone who opens the link will be told it was revoked. ${joined} This can't be undone.`;
}

/** The empty list's line, for each filter. */
export function emptyLine(filter: InviteFilter): string {
  return filter === "all"
    ? "No invites yet. Create one above to share a private link."
    : `No ${STATE_LABEL[filter].toLowerCase()} invites.`;
}
