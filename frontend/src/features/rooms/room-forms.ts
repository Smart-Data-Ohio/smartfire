/**
 * The room create and settings forms' logic: which channel types a person may pick, how a
 * draft becomes the API's `CreateRoom` / `UpdateRoom`, and how a picked emoji or icon becomes the
 * room's `iconName`. Pure, so it is tested apart from the dialogs.
 */
import type { CreateRoom } from "../../gen/CreateRoom.ts";
import type { RoomForm } from "../../gen/RoomForm.ts";
import type { RoomKind } from "../../gen/RoomKind.ts";
import type { UpdateRoom } from "../../gen/UpdateRoom.ts";
import type { EmojiData } from "../../lib/emoji/data.ts";
import type { IconName } from "../../ui/icons/icon.tsx";

/** A room kind the create dialog makes (direct rooms come from New message). */
export type ManagedKind = Exclude<RoomKind, "direct">;

/** What the create dialog offers: one text channel (public or private) and the call kinds. */
export type Channel = "text" | "voice" | "stage" | "board";

export interface ChannelOption {
  readonly channel: Channel;
  readonly title: string;
  readonly blurb: string;
  readonly icon: IconName;
}

export const CHANNEL_OPTIONS: readonly ChannelOption[] = [
  {
    channel: "text",
    title: "Text channel",
    blurb: "Messages, threads, files and pins",
    icon: "hash",
  },
  {
    channel: "voice",
    title: "Voice channel",
    blurb: "Drop in to talk or share a screen",
    icon: "volume",
  },
  {
    channel: "stage",
    title: "Stage",
    blurb: "Hosts speak while everyone listens",
    icon: "radio",
  },
  { channel: "board", title: "Board", blurb: "Posts and tasks for a team", icon: "boards" },
];

/** The classic page names for the new-room routes (`/app/rooms/new/<slug>`). */
export const NEW_ROOM_SLUGS = ["open", "closed", "voice", "stage", "board"] as const;

export type NewRoomSlug = (typeof NEW_ROOM_SLUGS)[number];

export function channelOf(kind: RoomKind): Channel {
  return kind === "open" || kind === "closed" || kind === "direct" ? "text" : kind;
}

export function kindOf(channel: Channel, isPrivate: boolean): ManagedKind {
  if (channel === "text") {
    return isPrivate ? "closed" : "open";
  }

  return channel;
}

/** The channels `allowed` (the form's `allowedTypes`) lets the viewer create, in display order. */
export function availableChannels(allowed: readonly RoomKind[]): readonly Channel[] {
  const set = new Set(allowed);

  return CHANNEL_OPTIONS.flatMap(({ channel }) =>
    (channel === "text" ? set.has("open") || set.has("closed") : set.has(channel)) ? [channel] : [],
  );
}

/** Whether `allowed` lets a text channel be public and private (so the switch is shown). */
export function privacyChoice(allowed: readonly RoomKind[]): "both" | "open" | "closed" {
  const open = allowed.includes("open");
  const closed = allowed.includes("closed");

  if (open && closed) return "both";

  return open ? "open" : "closed";
}

/** Everyone joins an open room; every other kind has a chosen member list. */
export function hasMemberList(kind: RoomKind): boolean {
  return kind !== "open" && kind !== "direct";
}

/** The fields a person edits. `name` as typed; `iconName` normalised, `null` for none. */
export interface RoomDraft {
  readonly name: string;
  readonly iconName: string | null;
  readonly userIds: readonly number[];
  /** Settings only; creation starts without a topic. */
  readonly topic?: string;
}

/** A blank name takes the kind's default (the form's `name`), as the classic form pre-fills. */
function nameOrDefault(name: string, fallback: string | null): string | null {
  const trimmed = name.trim();

  return trimmed === "" ? fallback : trimmed;
}

export function createBody(
  kind: ManagedKind,
  draft: RoomDraft,
  defaultName: string | null,
  clientRoomId: string,
): CreateRoom {
  const name = nameOrDefault(draft.name, defaultName);
  const userIds = [...new Set(draft.userIds)];
  const { iconName } = draft;

  switch (kind) {
    case "open":
      return { type: "open", clientRoomId, name, iconName };
    case "closed":
      return { type: "closed", clientRoomId, name, iconName, userIds };
    case "voice":
      return { type: "voice", clientRoomId, name, iconName, userIds };
    case "stage":
      return { type: "stage", clientRoomId, name, iconName, userIds };
    case "board":
      return { type: "board", clientRoomId, name, iconName, userIds };
  }
}

/**
 * The PATCH for `draft` against the loaded `form`: unchanged attributes are left out (so a legacy
 * icon the server no longer knows stays writable), and the member list is always the full set.
 */
export function updateBody(form: RoomForm, kind: ManagedKind, draft: RoomDraft): UpdateRoom {
  const name = draft.name.trim();
  const nameChange = name === (form.name ?? "") ? {} : { name: name === "" ? null : name };
  const iconChange = draft.iconName === form.iconName ? {} : { iconName: draft.iconName };
  const topic = draft.topic?.trim();

  const topicChange =
    topic === undefined || topic === (form.topic ?? "")
      ? {}
      : { topic: topic === "" ? null : topic };

  const changes = { ...nameChange, ...iconChange, ...topicChange };

  const userIds = [...new Set(draft.userIds)];

  switch (kind) {
    case "open":
      return { type: "open", ...changes };
    case "closed":
      return { type: "closed", ...changes, userIds };
    case "voice":
      return { type: "voice", ...changes, userIds };
    case "stage":
      return { type: "stage", ...changes, userIds };
    case "board":
      return { type: "board", ...changes, userIds };
  }
}

/**
 * Membership after the server added or removed people (the GitHub bot, on subscribe and on the
 * last unsubscribe). Local edits stay: someone the viewer added remains, someone they removed
 * stays removed, and only the server's own delta is applied.
 */
export function reconcileMembers(
  draftIds: readonly number[],
  before: readonly number[],
  after: readonly number[],
): number[] {
  const beforeIds = new Set(before);
  const afterIds = new Set(after);
  const removed = new Set<number>();

  for (const id of before) {
    if (!afterIds.has(id)) removed.add(id);
  }

  const next: number[] = [];

  for (const id of draftIds) {
    if (!removed.has(id) && !next.includes(id)) next.push(id);
  }

  for (const id of after) {
    if (!beforeIds.has(id) && !next.includes(id)) next.push(id);
  }

  return next;
}

/** Whether `draft` differs from what `form` loaded with. */
export function isDirty(form: RoomForm, kind: RoomKind, draft: RoomDraft): boolean {
  if (kind !== form.type) return true;

  if (draft.name.trim() !== (form.name ?? "")) return true;

  if (draft.iconName !== form.iconName) return true;

  if (draft.topic !== undefined && draft.topic.trim() !== (form.topic ?? "")) return true;

  if (!hasMemberList(kind)) return false;

  const before = new Set(form.userIds);
  const after = new Set(draft.userIds);

  return before.size !== after.size || [...after].some((id) => !before.has(id));
}

/**
 * A picked emoji or icon as the room's `iconName`: `:name:` loses its colons; an emoji character
 * becomes its first shortcode. `null` when the character has no shortcode the server knows.
 */
export function iconNameFor(content: string, emoji: EmojiData | null): string | null {
  const shortcode = /^:([a-z0-9_+-]+):$/.exec(content);

  if (shortcode?.[1] !== undefined) {
    return shortcode[1];
  }

  return emoji?.byChar.get(content)?.aliases[0] ?? null;
}

/** What a room's `iconName` draws as: an emoji character, an icon image, or nothing known. */
export type RoomIconLook =
  | { readonly kind: "emoji"; readonly char: string; readonly title: string }
  | { readonly kind: "image"; readonly url: string; readonly title: string }
  | { readonly kind: "none" };

export interface IconSource {
  readonly content: string;
  readonly title: string;
  readonly imageUrl: string | null;
}

export function roomIconLook(
  iconName: string | null,
  emoji: EmojiData | null,
  custom: readonly IconSource[],
): RoomIconLook {
  if (iconName === null || iconName === "") {
    return { kind: "none" };
  }

  const image = custom.find((icon) => icon.content === `:${iconName}:`);

  if (image?.imageUrl !== null && image?.imageUrl !== undefined) {
    return { kind: "image", url: image.imageUrl, title: image.title };
  }

  const entry = emoji?.byAlias.get(iconName);

  return entry === undefined
    ? { kind: "none" }
    : { kind: "emoji", char: entry.char, title: entry.name };
}

/** The room names the dialogs show in their titles: `#general`, or the name alone for calls. */
export function roomLabel(kind: RoomKind, name: string): string {
  return kind === "open" || kind === "closed" ? `#${name}` : name;
}
