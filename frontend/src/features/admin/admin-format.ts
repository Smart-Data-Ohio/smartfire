/** The admin screens' words and the small rules behind them. */
import type { AuditLogFilters } from "../../gen/AuditLogFilters.ts";
import type { HealthIssue } from "../../gen/HealthIssue.ts";
import type { IntegrationsHealth } from "../../gen/IntegrationsHealth.ts";
import type { Person } from "../../gen/Person.ts";
import type { Section } from "../settings/settings-format.ts";

/**
 * The admin sections, in the order the nav lists them; `path` is below `/app`. Everyone sees the
 * workspace and its people, as everyone can open the classic account page; the rest are for
 * administrators (`admin`).
 */
export const ADMIN_SECTIONS = [
  {
    key: "workspace",
    label: "Workspace",
    icon: "home",
    path: "/admin",
    page: "/admin/workspace",
    admin: false,
    group: "workspace",
  },
  {
    key: "invites",
    label: "Invites",
    icon: "user-plus",
    path: "/admin/invites",
    admin: true,
    group: "workspace",
  },
  {
    key: "people",
    label: "People",
    icon: "users",
    path: "/admin/people",
    admin: false,
    group: "workspace",
  },
  {
    key: "icons",
    label: "Workspace icons",
    icon: "smile",
    path: "/admin/icons",
    admin: true,
    group: "content",
  },
  {
    key: "sounds",
    label: "Soundboard",
    icon: "music",
    path: "/admin/sounds",
    admin: true,
    group: "content",
  },
  {
    key: "stickers",
    label: "Stickers",
    icon: "sticker",
    path: "/admin/stickers",
    admin: true,
    group: "content",
  },
  {
    key: "bots",
    label: "Chat bots",
    icon: "bot",
    path: "/admin/bots",
    admin: true,
    group: "content",
  },
  {
    key: "slack",
    label: "Slack import",
    icon: "cloud-upload",
    path: "/admin/slack",
    admin: true,
    group: "content",
  },
  {
    key: "styles",
    label: "Custom styles",
    icon: "code",
    path: "/admin/styles",
    admin: true,
    group: "content",
  },
  {
    key: "audit",
    label: "Audit log",
    icon: "file-text",
    path: "/admin/audit-log",
    admin: true,
    group: "health",
  },
  {
    key: "integrations",
    label: "Integration health",
    icon: "link",
    path: "/admin/integrations",
    admin: true,
    group: "health",
  },
] as const satisfies readonly (Section & { readonly admin: boolean })[];

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

/** The file types an icon upload accepts: still SVG and PNG, animated GIF and WebP. */
export const ICON_FILE_TYPES = "image/svg+xml,image/png,image/gif,image/webp";

/** How many animated icons the workspace holds of its limit; static icons don't count. */
export function animatedCapacity(usage: number, limit: number): string {
  const full = usage >= limit ? " The limit is reached; static icons are unlimited." : "";

  return `Animated icons: ${usage} of ${limit} used.${full}`;
}

const CAPACITY_REACHED = /^animated emoji capacity reached \(limit: (\d+)\)$/;

/**
 * An icon file's error as it shows under the field: the server's capacity message said plainly,
 * anything else as given.
 */
export function iconFileError(message: string | undefined): string | undefined {
  const limit = message === undefined ? undefined : CAPACITY_REACHED.exec(message)?.[1];

  return limit === undefined
    ? message
    : `The workspace already has ${limit} animated icons, its limit. Delete one, or upload a still SVG or PNG.`;
}

/** The audio files a soundboard sound may be, as the server accepts them. */
export const SOUND_FILE_TYPES = "audio/mpeg,audio/ogg,audio/wav,audio/x-wav,audio/webm";

/** How full the soundboard is: its sounds of the workspace's limit. */
export function soundCapacity(count: number, capacity: number): string {
  const full = count >= capacity ? " The soundboard is full: delete a sound to add another." : "";

  return `Sounds: ${count} of ${capacity} used.${full}`;
}

/** What each sound field is called in front of the server's message about it. */
const SOUND_FIELD_LABELS = new Map([
  ["name", "Name"],
  ["emoji", "Emoji"],
  ["volume", "Volume"],
  ["audio", "Audio file"],
]);

const SOUND_CAPACITY_REACHED = /^soundboard capacity reached \(limit: (\d+) sounds\)$/;

/**
 * A sound field's error as it shows: the server's `must be…` after the field's name, and its
 * capacity message said plainly.
 */
export function soundFieldError(field: string, message: string | undefined): string | undefined {
  if (message === undefined) {
    return undefined;
  }

  const limit = SOUND_CAPACITY_REACHED.exec(message)?.[1];

  if (limit !== undefined) {
    return `The soundboard already has ${limit} sounds, its limit. Delete one to add another.`;
  }

  if (field === "audio" && message === "isn't an uploaded file") {
    return "Choose an audio file to upload.";
  }

  const label = SOUND_FIELD_LABELS.get(field);

  return label === undefined ? message : `${label} ${message}.`;
}

/** A sound's default volume (0–1) as the list shows it. */
export function soundVolumeLabel(volume: number): string {
  return `${Math.round(Math.max(0, Math.min(1, volume)) * 100)}%`;
}

/** How full the sticker catalog is: its stickers of the workspace's limit. */
export function stickerCapacity(count: number, capacity: number): string {
  const full = count >= capacity ? " The catalog is full: delete a sticker to add another." : "";

  return `Stickers: ${count} of ${capacity} used.${full}`;
}

/** What the catalog's limit refusal (`StickerLimitReached`) says. */
export function stickerLimitMessage(capacity: number): string {
  return `The workspace already has ${capacity} stickers, its limit. Delete one to add another.`;
}

/** What each sticker field is called in front of the server's message about it. */
const STICKER_FIELD_LABELS = new Map([
  ["name", "Name"],
  ["description", "Description"],
  ["tags", "Tags"],
  ["image", "Image"],
]);

/** A sticker field's error as it shows: the server's `must be…` after the field's name. */
export function stickerFieldError(field: string, message: string | undefined): string | undefined {
  if (message === undefined) {
    return undefined;
  }

  if (
    field === "image" &&
    (message === "isn't an uploaded file" || message === "must be attached")
  ) {
    return "Choose an image to upload.";
  }

  const label = STICKER_FIELD_LABELS.get(field);

  return label === undefined ? message : `${label} ${message}.`;
}

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
