/** The admin screens' words and the small rules behind them. */
import type { AuditLogFilters } from "../../gen/AuditLogFilters.ts";
import type { HealthIssue } from "../../gen/HealthIssue.ts";
import type { IntegrationsHealth } from "../../gen/IntegrationsHealth.ts";
import type { Person } from "../../gen/Person.ts";
import type { IconName } from "../../ui/icons/icon.tsx";

/**
 * The admin sections, in the order the nav lists them; `path` is below `/app`. Everyone sees the
 * workspace and its people, as everyone can open the classic account page; the rest are for
 * administrators (`admin`).
 */
export const ADMIN_SECTIONS = [
  { key: "workspace", label: "Workspace", icon: "home", path: "/admin", admin: false },
  { key: "people", label: "People", icon: "users", path: "/admin/people", admin: false },
  { key: "icons", label: "Workspace icons", icon: "smile", path: "/admin/icons", admin: true },
  { key: "styles", label: "Custom styles", icon: "code", path: "/admin/styles", admin: true },
  { key: "audit", label: "Audit log", icon: "file-text", path: "/admin/audit-log", admin: true },
  {
    key: "integrations",
    label: "Integration health",
    icon: "link",
    path: "/admin/integrations",
    admin: true,
  },
] as const satisfies readonly {
  readonly key: string;
  readonly label: string;
  readonly icon: IconName;
  readonly path: string;
  readonly admin: boolean;
}[];

/** The sections someone sees: all of them for an administrator, the open ones otherwise. */
export function visibleSections(canAdminister: boolean) {
  return ADMIN_SECTIONS.filter((section) => canAdminister || !section.admin);
}

/** The people list as the classic page groups it: administrators, then members. */
export function groupPeople(people: readonly Person[]) {
  return {
    administrators: people.filter((person) => person.role === "administrator"),
    members: people.filter((person) => person.role === "member"),
  };
}

/** The row to move to when `id` leaves `ids`: the next one, else the previous, else none. */
export function neighbour<T>(ids: readonly T[], id: T): T | null {
  const index = ids.indexOf(id);

  if (index === -1) {
    return null;
  }

  return ids[index + 1] ?? ids[index - 1] ?? null;
}

/** The classic page's confirmation before a two-step sign-in reset. */
export function twoFactorResetConfirmation(person: Person): string {
  return `Reset two-step sign-in for ${person.name}? They will sign out everywhere and set it up again at next sign-in.`;
}

/** The classic page's title for allowing Google sign-in by email. */
export function googleLinkTitle(person: Person): string {
  return `Allow Google sign-in to link ${person.emailAddress ?? ""}`;
}

/** The classic page's confirmation before Google sign-in is allowed by email. */
export function googleLinkConfirmation(person: Person): string {
  return `${person.name} chose the email ${person.emailAddress ?? ""} themselves. Allow the Google account with that address to sign in as them?`;
}

/** The classic page's title for unlinking Google sign-in. */
export function googleUnlinkTitle(email: string): string {
  return `Unlink Google sign-in (${email})`;
}

/** The classic page's confirmation before Google sign-in is unlinked. */
export function googleUnlinkConfirmation(person: Person): string {
  return `Unlink Google sign-in from ${person.name}? That Google account will no longer sign in as them.`;
}

/** What the classic page asks before someone is removed. */
export const REMOVE_CONFIRMATION =
  "Are you sure you want to permanently remove this person from the account? This can’t be undone.";

/** The classic page's rule for icon names. */
export const ICON_NAME_HINT = "lowercase letters, numbers, underscores, 2–32 characters";

/** No filters set. */
export const NO_FILTERS: AuditLogFilters = {
  actor: null,
  action: null,
  targetType: null,
  from: null,
  to: null,
};

/** A form value as a filter: blank is unset. */
export function filterValue(value: string): string | null {
  const trimmed = value.trim();

  return trimmed === "" ? null : trimmed;
}

/**
 * The audit log's time column: the entry's moment in the viewer's locale, read in `timeZone` (the
 * zone the log's date filters use) or the browser's own zone when it's absent.
 */
export function auditTime(createdAt: string, timeZone?: string, locale?: string): string {
  return new Date(createdAt).toLocaleString(locale, {
    timeZone,
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  });
}

/** A labelled fact on the health page: "Webhook secret", "Set". */
export type HealthFact = readonly [label: string, value: string];

/** The classic health page's facts and sentences for each integration. */
export interface HealthFacts {
  readonly github: readonly HealthFact[];
  readonly google: readonly HealthFact[];
  readonly expiring: readonly HealthIssue[];
  readonly fizzy: string;
  readonly delivery: readonly HealthFact[];
  readonly email: string;
}

/** The classic health page's facts and sentences, from the snapshot. */
export function healthFacts(health: IntegrationsHealth): HealthFacts {
  const { github, google, fizzy, agentDelivery, email } = health;
  const rooms = email.roomsWithAddresses;

  return {
    github: [
      [
        "Workspace token",
        github.workspaceToken ? "Set" : "Not set — private cards need per-user accounts",
      ],
      [
        "GitHub App",
        github.appConfigured
          ? "Configured"
          : "Not configured — set GITHUB_APP_CLIENT_ID and GITHUB_APP_CLIENT_SECRET (see docs/github-app.md)",
      ],
      ["Webhook secret", github.webhookSecret ? "Set" : "Not set — cards refresh on render only"],
      [
        "Connected accounts",
        `${github.connected} (${github.appTokens} App, ${github.connected - github.appTokens} PAT)`,
      ],
      ["Webhook deliveries (24h)", `${github.deliveries24h}`],
    ],
    google: [
      [
        "OAuth client",
        google.configured
          ? "Configured"
          : "Not configured — set GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET",
      ],
      ["Connected accounts", `${google.connected}`],
      [
        "Push channels",
        google.pushEnabled
          ? `${google.pushChannels} watching`
          : "Disabled — set GOOGLE_CALENDAR_WEBHOOK_URL for two-way RSVP sync (see docs/meet-and-rsvp.md)",
      ],
    ],
    expiring: google.expiring.map((channel) => ({
      subject: `user ${channel.userId}`,
      detail: `expires ${channel.expiresAt === null ? "unknown" : auditTime(channel.expiresAt)}${
        channel.error === null ? "" : ` — ${channel.error}`
      }`,
    })),
    fizzy: fizzy.configured ? "Configured" : (fizzy.note ?? ""),
    delivery: [
      ["Agent webhooks pending", `${agentDelivery.pending}`],
      ["Agent webhooks failed (24h)", `${agentDelivery.failed24h}`],
    ],
    email: email.enabled
      ? `${rooms} ${rooms === 1 ? "room" : "rooms"} with a forward-to address.`
      : "Inbound email is not configured. Set INBOUND_EMAIL_DOMAIN and the relay ingress password (see docs/email-to-room.md).",
  };
}
