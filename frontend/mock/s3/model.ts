/**
 * What the mock keeps for S3 beyond the S2 world (the viewer's activity inbox), and the pure
 * helpers the S3 modules share: item state, keyset paging and the names a cross-room list gives
 * its conversations.
 */
import type { ActivityItem } from "../../src/gen/ActivityItem.ts";
import type { ActivityState } from "../../src/gen/ActivityState.ts";
import { validation } from "../http.ts";

/** The S3 part of the world, mutated in place by the S3 modules. */
export interface S3World {
  /** The viewer's inbox, by item id. */
  readonly activity: Map<number, ActivityItem>;
  nextActivityId: number;
  activityRevision: number;
}

/** A fresh, empty S3 world. */
export function emptyS3World(): S3World {
  return { activity: new Map(), nextActivityId: 1, activityRevision: 0 };
}

/** `ActivityItem#state`: handled when `handledAt` is set, else read when `readAt` is. */
export function activityStateOf(item: Pick<ActivityItem, "readAt" | "handledAt">): ActivityState {
  if (item.handledAt !== null) return "handled";

  return item.readAt === null ? "unread" : "read";
}

/** The item with `state` derived from its timestamps again. */
export function withActivityState(item: ActivityItem): ActivityItem {
  return { ...item, state: activityStateOf(item) };
}

/** A row's place in a keyset order: a timestamp, then the id. */
export interface KeysetKey {
  readonly at: string;
  readonly id: number;
}

/** Sorts newest first (`at DESC, id DESC`), or oldest first with `ascending`. */
export function compareKeys(left: KeysetKey, right: KeysetKey, ascending = false): number {
  const order = left.at === right.at ? left.id - right.id : left.at < right.at ? -1 : 1;

  return ascending ? order : -order;
}

/** One page of a keyset-ordered list. */
export interface KeysetPage<T> {
  readonly rows: readonly T[];
  /** The opaque cursor of the page's last row when more rows follow it; `null` on the last. */
  readonly nextCursor: string | null;
}

/** The opaque cursor the server hands out: base64url of `<at>|<id>`. */
export function encodeCursor(key: KeysetKey): string {
  return btoa(`${key.at}|${key.id}`).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

/** The key a cursor names; 422 when it doesn't decode. */
export function decodeCursor(cursor: string): KeysetKey {
  const invalid = () => validation("before", "Before is not a valid cursor");
  let text: string;

  try {
    text = atob(cursor.replace(/-/g, "+").replace(/_/g, "/"));
  } catch {
    throw invalid();
  }

  const bar = text.lastIndexOf("|");
  const at = text.slice(0, bar);
  const id = Number(text.slice(bar + 1));

  if (bar < 0 || Number.isNaN(Date.parse(at)) || !Number.isInteger(id)) throw invalid();

  return { at, id };
}

/**
 * The page after `before` in `rows` (already filtered, any order): sorts by `keyOf`, skips past
 * the cursor's key and takes `limit`, reading one more to know whether another page exists. The
 * cursor carries the key itself, so a row that moved state while the client paged can't lose
 * its place.
 */
export function keysetPage<T>(
  rows: readonly T[],
  keyOf: (row: T) => KeysetKey,
  before: string | null,
  limit: number,
  ascending = false,
): KeysetPage<T> {
  const sorted = [...rows].sort((left, right) => compareKeys(keyOf(left), keyOf(right), ascending));
  const cursor = before === null ? null : decodeCursor(before);

  const after =
    cursor === null
      ? sorted
      : sorted.filter((row) => compareKeys(keyOf(row), cursor, ascending) > 0);

  const page = after.slice(0, limit);
  const last = page.at(-1);

  return {
    rows: page,
    nextCursor: after.length > limit && last !== undefined ? encodeCursor(keyOf(last)) : null,
  };
}

/** The `before` query parameter, or `null`. */
export function beforeOf(query: URLSearchParams): string | null {
  const raw = query.get("before");

  return raw === null || raw === "" ? null : raw;
}

/** The first of `allowed` that `raw` names, else `fallback` (unknown values read as the default). */
export function oneOf<T extends string>(raw: string | null, allowed: readonly T[], fallback: T): T {
  return allowed.find((candidate) => candidate === raw) ?? fallback;
}

/** The inbox's body limit: 500 characters, the last three `...` when cut. */
export function truncateBody(text: string): string {
  const chars = [...text];

  return chars.length > 500 ? `${chars.slice(0, 497).join("")}...` : text;
}

/** A message's plain text, near enough to `plain_text_body`: its Markdown without the markup. */
export function plainText(markdown: string | null): string {
  if (markdown === null) return "";

  return markdown
    .replace(/```[a-z]*\n?/g, "")
    .replace(/!?\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/[*_`~]+/g, "")
    .replace(/^>\s?/gm, "")
    .replace(/\s+/g, " ")
    .trim();
}
