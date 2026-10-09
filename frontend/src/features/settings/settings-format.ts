/** The settings screens' words and the small rules behind their forms. */
import type { LinkProps } from "@tanstack/react-router";
import type { Connection } from "../../gen/Connection.ts";
import type { IntegrationChange } from "../../gen/IntegrationChange.ts";
import type { OooPreset } from "../../gen/OooPreset.ts";
import type { PresenceSetting } from "../../gen/PresenceSetting.ts";
import type { PushSubscriptionInfo } from "../../gen/PushSubscriptionInfo.ts";
import type { RememberedDevice } from "../../gen/RememberedDevice.ts";
import type { SessionInfo } from "../../gen/SessionInfo.ts";
import type { Settings } from "../../gen/Settings.ts";
import type { StatusExpiry } from "../../gen/StatusExpiry.ts";
import type { StatusSettings } from "../../gen/StatusSettings.ts";
import type { TextSize } from "../../gen/TextSize.ts";
import type { Theme } from "../../gen/Theme.ts";
import type { VoiceMode } from "../../gen/VoiceMode.ts";
import type { IconName } from "../../ui/icons/icon.tsx";
import { timeAgo } from "../threads/thread-format.ts";

/** Where a section link can go: a route below `/app`. */
export type SectionPath = NonNullable<LinkProps["to"]>;

/** A settings or workspace section, as its nav (and on phones, its list) shows it. */
export interface Section {
  readonly key: string;
  readonly label: string;
  readonly icon: IconName;
  /** Where the nav links, below `/app`. */
  readonly path: SectionPath;
  /**
   * The section's own page, for the one whose `path` is the root (`/settings`): on phones the
   * root is the list of sections, so the first section is pushed over it from here instead.
   */
  readonly page?: SectionPath;
  /** Pages under it that live at another path (the personal Slack import, under integrations). */
  readonly under?: SectionPath;
  /** Neighbours that share a group share a block in the phone list. */
  readonly group: string;
}

/** The settings sections, in the order the nav lists them; `path` is below `/app/settings`. */
export const SECTIONS = [
  {
    key: "profile",
    label: "Profile",
    icon: "at",
    path: "/settings",
    page: "/settings/profile",
    group: "you",
  },
  { key: "status", label: "Status", icon: "smile", path: "/settings/status", group: "you" },
  {
    key: "notifications",
    label: "Notifications",
    icon: "bell",
    path: "/settings/notifications",
    group: "alerts",
  },
  { key: "rooms", label: "Rooms", icon: "hash", path: "/settings/rooms", group: "alerts" },
  {
    key: "appearance",
    label: "Appearance",
    icon: "sun",
    path: "/settings/appearance",
    group: "app",
  },
  { key: "calls", label: "Calls", icon: "headphones", path: "/settings/calls", group: "app" },
  {
    key: "security",
    label: "Security",
    icon: "shield",
    path: "/settings/security",
    group: "account",
  },
  {
    key: "sessions",
    label: "Sessions",
    icon: "lock",
    path: "/settings/sessions",
    group: "account",
  },
  {
    key: "devices",
    label: "Push devices",
    icon: "monitor",
    path: "/settings/devices",
    group: "account",
  },
  {
    key: "integrations",
    label: "Integrations",
    icon: "link",
    path: "/settings/integrations",
    under: "/settings/slack",
    group: "integrations",
  },
] as const satisfies readonly Section[];

/** Sections in runs of the same group, in order: the phone list's blocks. */
export function groupSections<S extends Section>(sections: readonly S[]): S[][] {
  const groups: S[][] = [];

  for (const section of sections) {
    const last = groups.at(-1);

    if (last !== undefined && last[0]?.group === section.group) {
      last.push(section);
    } else {
      groups.push([section]);
    }
  }

  return groups;
}

export type SectionKey = (typeof SECTIONS)[number]["key"];

/** A labelled choice for a radio group or select. */
export interface Choice<T extends string> {
  readonly value: T;
  readonly label: string;
}

export const THEME_CHOICES: readonly Choice<Theme>[] = [
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
  { value: "system", label: "System" },
];

export const TEXT_SIZE_CHOICES: readonly Choice<TextSize>[] = [
  { value: "smaller", label: "Smaller" },
  { value: "small", label: "Small" },
  { value: "default", label: "Default" },
  { value: "large", label: "Large" },
  { value: "larger", label: "Larger" },
];

export const PRESENCE_CHOICES: readonly Choice<PresenceSetting>[] = [
  { value: "auto", label: "Automatic" },
  { value: "dnd", label: "Do not disturb" },
  { value: "invisible", label: "Invisible (appear offline)" },
];

export const EXPIRY_CHOICES: readonly Choice<StatusExpiry>[] = [
  { value: "minutes_30", label: "30 minutes" },
  { value: "hour_1", label: "1 hour" },
  { value: "hours_4", label: "4 hours" },
  { value: "today", label: "Today" },
  { value: "week", label: "This week" },
  { value: "never", label: "Never" },
];

/** The out-of-office end presets; "" leaves the end as it is (the classic "Don't change"). */
export const OOO_CHOICES: readonly Choice<OooPreset | "">[] = [
  { value: "", label: "Don't change" },
  { value: "tomorrow", label: "Until tomorrow" },
  { value: "monday", label: "Until Monday" },
  { value: "week", label: "1 week" },
  { value: "custom", label: "Custom date and time" },
];

export const VOICE_CHOICES: readonly Choice<VoiceMode>[] = [
  { value: "voice_activity", label: "Voice activity" },
  { value: "push_to_talk", label: "Push to talk" },
];

/** The classic page's keyword limit ("up to 20"); the server enforces it. */
export const MAX_KEYWORDS = 20;

/** Keyword alerts from the textarea: one per line, trimmed, blanks dropped. */
export function keywordLines(text: string): string[] {
  return text
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line !== "");
}

/**
 * A field's messages as the classic forms print them: "Current password is incorrect." (the
 * label, then each message joined with "and"); `undefined` when the field has none.
 */
export function fieldError(
  fields: Readonly<Record<string, readonly string[]>>,
  field: string,
  label: string,
): string | undefined {
  const messages = fields[field] ?? [];

  return messages.length === 0 ? undefined : `${label} ${messages.join(" and ")}.`;
}

const dateTime = new Intl.DateTimeFormat(undefined, {
  weekday: "short",
  month: "short",
  day: "numeric",
  hour: "numeric",
  minute: "2-digit",
});

const shortDate = new Intl.DateTimeFormat(undefined, {
  month: "short",
  day: "numeric",
  year: "numeric",
});

/** "Out of office until Fri, Oct 9, 5:00 PM", from your calendar when it isn't manual. */
export function oooSummary(status: StatusSettings): string | null {
  if (status.oooUntil === null) {
    return null;
  }

  const until = dateTime.format(Date.parse(status.oooUntil));

  return status.oooManual
    ? `Out of office until ${until}.`
    : `Out of office until ${until} from your Google Calendar.`;
}

/** "Clears Fri, Oct 9, 5:00 PM", or `null` for a status that stays. */
export function statusExpiry(status: StatusSettings): string | null {
  return status.customStatusExpiresAt === null
    ? null
    : `Clears ${dateTime.format(Date.parse(status.customStatusExpiresAt))}`;
}

/** "Last active 5 minutes ago · 203.0.113.9 · signed in Oct 1, 2026", the classic line. */
export function sessionMeta(session: SessionInfo, now: number): string {
  const parts = [`Last active ${timeAgo(session.lastActiveAt, now)}`];

  if (session.ipAddress !== null) {
    parts.push(session.ipAddress);
  }

  parts.push(`signed in ${shortDate.format(Date.parse(session.createdAt))}`);

  return parts.join(" · ");
}

/** "Chrome 141 on Android". */
export function deviceName(subscription: PushSubscriptionInfo): string {
  return `${subscription.browser} ${subscription.version} on ${subscription.platform}`.trim();
}

/**
 * Where a GitHub or Fizzy connection stands, as the classic panel's first line puts it; `null`
 * when there's none (the panel explains what connecting does instead).
 */
export function connectionSummary(
  service: "GitHub" | "Fizzy",
  connection: Connection,
): string | null {
  switch (connection.state) {
    case "connected": {
      const where = connection.workspace === null ? "" : ` (${connection.workspace})`;
      const app = connection.appToken ? " through the GitHub App" : "";

      return `Connected as ${connection.name}${where}${app}.`;
    }

    case "rejected": {
      const reason = connection.reason === null ? "" : ` (${connection.reason})`;
      const next = service === "GitHub" ? "Reconnect below." : "Paste a new token to reconnect.";

      return `${service} rejected the connection${reason}. ${next}`;
    }

    case "missing":
      return null;
  }
}

/** `path` with `?classic=1`, so a person who uses the new UI stays on the classic page. */
export function classicPage(path: string, anchor = ""): string {
  const [base, query] = path.split("?", 2);
  const params = new URLSearchParams(query ?? "");

  params.set("classic", "1");

  return `${base}?${params.toString()}${anchor === "" ? "" : `#${anchor}`}`;
}

/**
 * The page after a change to `service`: only that service's connection is taken from the answer,
 * so two answers settling out of order never undo each other's service.
 */
export function withConnection(
  current: Settings,
  service: "github" | "fizzy" | "google",
  change: IntegrationChange,
): Settings {
  return {
    ...current,
    integrations: { ...current.integrations, [service]: change.integrations[service] },
  };
}

/**
 * The parts of the page a connection change also moves, from a fresh load: GitHub sets (or frees)
 * the profile's GitHub username, and dropping Google Calendar deletes the meeting cache behind
 * the status's calendar out-of-office. The connections themselves stay as the change answers left
 * them.
 */
export function withDependents(current: Settings, fresh: Settings): Settings {
  return { ...current, profile: fresh.profile, status: fresh.status };
}

const longDay = new Intl.DateTimeFormat(undefined, {
  month: "long",
  day: "numeric",
  year: "numeric",
});

/** "On since October 7, 2026.", the classic two-step panel's first line. */
export function twoFactorSince(confirmedAt: string): string {
  return `On since ${longDay.format(Date.parse(confirmedAt))}.`;
}

/**
 * "203.0.113.9 · last used 3 hours ago" for a remembered browser, as the classic row's second line
 * reads; `null` when it knows neither.
 */
export function rememberedMeta(device: RememberedDevice, now: number): string | null {
  const parts = [
    ...(device.ipAddress === null ? [] : [device.ipAddress]),
    ...(device.lastUsedAt === null ? [] : [`last used ${timeAgo(device.lastUsedAt, now)}`]),
  ];

  return parts.length === 0 ? null : parts.join(" · ");
}

/** What the confirmation field asks for: a password only when the account has one. */
export function reauthLabel(hasPassword: boolean): string {
  return hasPassword ? "Authenticator code or password" : "Authenticator code";
}

/**
 * An SVG document as an image source that never leaves the page: a `data:` URL, so the sign-in
 * link it encodes reaches no request path, log or cache.
 */
export function svgDataUrl(svg: string): string {
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}
