/**
 * What the search field suggests as you type, Slack's way. Empty: your recent searches and the
 * filters you can narrow by. Typing words: search for them, matching recent searches, and the
 * people and channels whose names match (from the switcher's data). Typing an operator
 * (`from:`, `in:`, `has:`, `is:`, a date): the values that finish it. Pure; the field renders it.
 */
import type { RecentSearch } from "../../gen/RecentSearch.ts";
import type { SearchOperator } from "../../gen/SearchOperator.ts";
import type { RoomKind } from "../../store/model.ts";
import type { IconName } from "../../ui/icons/icon.tsx";
import { fold, matchScore, normalizeQuery } from "../switcher/match.ts";
import type { SwitcherItem } from "../switcher/ranking.ts";
import { isoDay, OPERATORS, operatorPrefixes, partialOperator, replaceLastToken } from "./query.ts";

/** What choosing a suggestion does. */
export type TypeaheadAction =
  /** Searches for `query` (and remembers it). */
  | { readonly kind: "search"; readonly query: string }
  /** Puts `value` in the field and keeps suggesting (an operator or its value). */
  | { readonly kind: "complete"; readonly value: string }
  /** Opens a conversation (or a person's DM, made on demand when `roomId` is `null`). */
  | {
      readonly kind: "open";
      readonly roomId: number | null;
      readonly userId: number | null;
    };

export interface TypeaheadItem {
  readonly key: string;
  readonly label: string;
  /** Faint text after the label (an operator's syntax, a channel's kind). */
  readonly detail: string | null;
  readonly icon: IconName | null;
  /** A person's avatar instead of an icon. */
  readonly userId: number | null;
  readonly roomKind: RoomKind | null;
  readonly action: TypeaheadAction;
}

export type TypeaheadSectionKey =
  | "query"
  | "recent"
  | "operators"
  | "values"
  | "people"
  | "channels";

export interface TypeaheadSection {
  readonly key: TypeaheadSectionKey;
  /** `null` for the untitled "Search for …" row. */
  readonly title: string | null;
  readonly items: readonly TypeaheadItem[];
}

/** What the suggestions are drawn from. */
export interface TypeaheadSource {
  readonly recents: readonly RecentSearch[];
  /** The switcher's merged items: people (with names) and rooms. */
  readonly items: readonly SwitcherItem[];
  /** The clock, for date suggestions. */
  readonly now: number;
}

const EMPTY_RECENTS = 6;

const MATCHING_RECENTS = 3;

const PER_KIND = 4;

const VALUES_SHOWN = 6;

const DAY_MS = 86_400_000;

/** Each operator as the "Narrow your search" list explains it. */
export const OPERATOR_HINTS = {
  from: { token: "from:", label: "From a person", icon: "at" },
  in: { token: "in:", label: "In a channel", icon: "hash" },
  has: { token: "has:", label: "Has a link, file, image or pin", icon: "paperclip" },
  is: { token: "is:thread", label: "Only replies in threads", icon: "thread" },
  before: { token: "before:", label: "Before a date", icon: "calendar-clock" },
  after: { token: "after:", label: "After a date", icon: "calendar-clock" },
  on: { token: "on:", label: "On a date", icon: "calendar-clock" },
} as const satisfies Record<
  SearchOperator,
  { readonly token: string; readonly label: string; readonly icon: IconName }
>;

const HAS_VALUES = [
  { value: "link", label: "Links", icon: "link" },
  { value: "file", label: "Files", icon: "file" },
  { value: "image", label: "Images", icon: "image" },
  { value: "pin", label: "Pinned messages", icon: "pin" },
] as const satisfies readonly {
  readonly value: string;
  readonly label: string;
  readonly icon: IconName;
}[];

const BLANK = { detail: null, icon: null, userId: null, roomKind: null } as const;

function section(
  key: TypeaheadSectionKey,
  title: string | null,
  items: readonly TypeaheadItem[],
): TypeaheadSection[] {
  return items.length === 0 ? [] : [{ key, title, items }];
}

function recentItem(search: RecentSearch): TypeaheadItem {
  return {
    ...BLANK,
    key: `recent:${search.id}:${search.query}`,
    label: search.query,
    icon: "clock",
    action: { kind: "search", query: search.query },
  };
}

function operatorItem(value: string, operator: SearchOperator): TypeaheadItem {
  const hint = OPERATOR_HINTS[operator];

  return {
    ...BLANK,
    key: `operator:${operator}`,
    label: hint.token,
    detail: hint.label,
    icon: hint.icon,
    action: { kind: "complete", value: replaceLastToken(value, hint.token).trimEnd() },
  };
}

/** The person's first name, lowercased: what `from:@` takes (a name holds no spaces there). */
export function fromHandle(name: string): string {
  return fold(name.split(/\s+/u)[0] ?? name);
}

/** A channel name as `in:#` takes it (no spaces either). */
function inHandle(name: string): string {
  return name.replace(/\s+/gu, "-").toLowerCase();
}

function isPerson(item: SwitcherItem): boolean {
  return item.kind === "person" && item.userId !== null;
}

function isChannel(item: SwitcherItem): boolean {
  return item.kind === "room" && item.roomKind !== null && item.roomKind !== "direct";
}

/** The best `limit` items of a kind for `needle`, best first. */
function best(
  items: readonly SwitcherItem[],
  needle: string,
  keep: (item: SwitcherItem) => boolean,
  limit: number,
): SwitcherItem[] {
  return items
    .flatMap((item) => {
      const score = keep(item) ? matchScore(item.label, needle) : null;

      return score === null ? [] : [{ item, score }];
    })
    .sort(
      (left, right) => right.score - left.score || left.item.label.localeCompare(right.item.label),
    )
    .slice(0, limit)
    .map((entry) => entry.item);
}

function valueItem(
  value: string,
  key: string,
  label: string,
  token: string,
  extra: Partial<TypeaheadItem>,
): TypeaheadItem {
  return {
    ...BLANK,
    key,
    label,
    detail: token,
    ...extra,
    action: { kind: "complete", value: replaceLastToken(value, token) },
  };
}

function dateValues(
  value: string,
  operator: SearchOperator,
  partial: string,
  now: number,
): TypeaheadItem[] {
  const days = [
    { label: "Today", millis: now },
    { label: "Yesterday", millis: now - DAY_MS },
    { label: "A week ago", millis: now - 7 * DAY_MS },
    { label: "A month ago", millis: now - 30 * DAY_MS },
  ];

  return days.flatMap((day) => {
    const token = `${operator}:${isoDay(day.millis)}`;

    return token.startsWith(`${operator}:${partial}`) || fold(day.label).startsWith(fold(partial))
      ? [valueItem(value, `date:${day.label}`, day.label, token, { icon: "calendar-clock" })]
      : [];
  });
}

/** The values that finish the operator being typed. */
function operatorValues(
  value: string,
  operator: SearchOperator,
  partial: string,
  source: TypeaheadSource,
): TypeaheadItem[] {
  const needle = normalizeQuery(partial);

  switch (operator) {
    case "from":
      return best(source.items, needle, isPerson, VALUES_SHOWN).map((item) =>
        valueItem(value, `from:${item.key}`, item.label, `from:@${fromHandle(item.label)}`, {
          userId: item.userId,
        }),
      );

    case "in":
      return best(source.items, needle, isChannel, VALUES_SHOWN).map((item) =>
        valueItem(value, `in:${item.key}`, item.label, `in:#${inHandle(item.label)}`, {
          roomKind: item.roomKind,
        }),
      );

    case "has":
      return HAS_VALUES.flatMap((entry) =>
        entry.value.startsWith(fold(partial)) || fold(entry.label).startsWith(fold(partial))
          ? [
              valueItem(value, `has:${entry.value}`, entry.label, `has:${entry.value}`, {
                icon: entry.icon,
              }),
            ]
          : [],
      );

    case "is":
      return "thread".startsWith(fold(partial))
        ? [valueItem(value, "is:thread", "Thread replies", "is:thread", { icon: "thread" })]
        : [];

    default:
      return dateValues(value, operator, partial, source.now);
  }
}

const VALUE_TITLES = {
  from: "From",
  in: "In",
  has: "Has",
  is: "Only",
  before: "Before",
  after: "After",
  on: "On",
} as const satisfies Record<SearchOperator, string>;

/** The suggestions for the field's `value`. */
export function typeaheadSections(value: string, source: TypeaheadSource): TypeaheadSection[] {
  const query = value.trim();

  if (query === "") {
    return [
      ...section(
        "recent",
        "Recent searches",
        source.recents.slice(0, EMPTY_RECENTS).map(recentItem),
      ),
      ...section(
        "operators",
        "Narrow your search",
        OPERATORS.map((operator) => operatorItem(value, operator)),
      ),
    ];
  }

  const typing = partialOperator(value);

  if (typing !== null) {
    return section(
      "values",
      VALUE_TITLES[typing.operator],
      operatorValues(value, typing.operator, typing.partial, source),
    );
  }

  const folded = fold(query);

  const recents = source.recents
    .filter((search) => fold(search.query).includes(folded) && search.query !== query)
    .slice(0, MATCHING_RECENTS)
    .map(recentItem);

  const needle = normalizeQuery(query);

  const people = best(source.items, needle, isPerson, PER_KIND).map(
    (item): TypeaheadItem => ({
      ...BLANK,
      key: `person:${item.key}`,
      label: item.label,
      detail: item.roomId === null ? "New message" : null,
      userId: item.userId,
      action: { kind: "open", roomId: item.roomId, userId: item.userId },
    }),
  );

  const channels = best(source.items, needle, isChannel, PER_KIND).map(
    (item): TypeaheadItem => ({
      ...BLANK,
      key: `channel:${item.key}`,
      label: item.label,
      roomKind: item.roomKind,
      action: { kind: "open", roomId: item.roomId, userId: null },
    }),
  );

  const searchRow: TypeaheadItem = {
    ...BLANK,
    key: "query",
    label: query,
    icon: "search",
    action: { kind: "search", query },
  };

  return [
    ...section("query", null, [searchRow]),
    ...section(
      "operators",
      "Narrow your search",
      operatorPrefixes(value).map((operator) => operatorItem(value, operator)),
    ),
    ...section("recent", "Recent searches", recents),
    ...section("people", "People", people),
    ...section("channels", "Channels", channels),
  ];
}

/** The sections' items in display order, for ↑/↓. */
export function flattenTypeahead(sections: readonly TypeaheadSection[]): TypeaheadItem[] {
  return sections.flatMap((entry) => entry.items);
}
