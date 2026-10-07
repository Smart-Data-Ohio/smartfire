/** Words and small rules for boards: statuses, owners, filters, tags. */
import type { BoardStatusFilter } from "../../gen/BoardStatusFilter.ts";
import type { User } from "../../gen/User.ts";
import type { WorkFacts } from "../../gen/WorkFacts.ts";
import type { WorkStatus } from "../../gen/WorkStatus.ts";
import type { BoardSearch } from "../../lib/board-search.ts";

export const WORK_STATUSES = [
  "planned",
  "in_progress",
  "blocked",
  "done",
] as const satisfies readonly WorkStatus[];

export const WORK_STATUS_LABEL = {
  planned: "Planned",
  in_progress: "In progress",
  blocked: "Blocked",
  done: "Done",
} as const satisfies Record<WorkStatus, string>;

export const STATUS_FILTER_LABEL = {
  open: "Open",
  done: "Done",
  all: "All",
} as const satisfies Record<BoardStatusFilter, string>;

export type BoardView = "list" | "board";

/** The board's query as the URL carries it, with the classic defaults filled in. */
export interface BoardQuery {
  readonly view: BoardView;
  readonly status: BoardStatusFilter;
  readonly owner: string;
  readonly tag: string;
}

/** The column view always lists every status, as the classic board does. */
export function boardQuery(search: BoardSearch): BoardQuery {
  const view = search.view ?? "list";

  return {
    view,
    status: view === "board" ? "all" : (search.status ?? "open"),
    owner: search.owner ?? "anyone",
    tag: search.tag ?? "",
  };
}

/** The URL's search for a query: defaults left out, so a plain board stays `/r/:id`. */
export function boardSearch(query: BoardQuery): BoardSearch {
  const search: { -readonly [Key in keyof BoardSearch]: BoardSearch[Key] } = {};

  if (query.view === "board") {
    search.view = "board";
  } else if (query.status !== "open") {
    search.status = query.status;
  }

  if (query.owner !== "anyone") {
    search.owner = query.owner;
  }

  if (query.tag !== "") {
    search.tag = query.tag;
  }

  return search;
}

/** How many filters differ from the defaults (the status filter only counts in the list). */
export function activeFilterCount(query: BoardQuery): number {
  return (
    (query.view === "list" && query.status !== "open" ? 1 : 0) +
    (query.owner === "anyone" ? 0 : 1) +
    (query.tag === "" ? 0 : 1)
  );
}

/** "Me", "Agents", or the member's name ("Someone" until they load). */
export function ownerFilterLabel(owner: string, users: Readonly<Record<number, User>>): string {
  if (owner === "me") {
    return "Me";
  }

  if (owner === "agents") {
    return "Agents";
  }

  if (owner === "anyone") {
    return "Anyone";
  }

  return users[Number(owner)]?.name ?? "Someone";
}

/**
 * A post's owner line: "Unassigned", the owner's name, or "Owner unavailable (Name)" once they
 * can't act on it (left, deactivated, or an agent that may no longer post there).
 */
export function ownerLabel(work: WorkFacts): string {
  if (work.owner === null) {
    return "Unassigned";
  }

  return work.ownerActive ? work.owner.name : `Owner unavailable (${work.owner.name})`;
}

export const MAX_TAGS = 5;

export const MAX_TAG_LENGTH = 30;

const TAG_GRAMMAR = /^[a-z0-9][a-z0-9-]*$/;

/** "bug, API ,bug" → ["bug", "api"]: stripped, lower-cased, blanks and repeats dropped. */
export function parseTags(text: string): string[] {
  const tags: string[] = [];

  for (const part of text.split(",")) {
    const tag = part.trim().toLowerCase();

    if (tag !== "" && !tags.includes(tag)) {
      tags.push(tag);
    }
  }

  return tags;
}

/** What the server would refuse about these tags, said the same way; `undefined` when fine. */
export function tagsProblem(tags: readonly string[]): string | undefined {
  if (tags.length > MAX_TAGS) {
    return `Use up to ${MAX_TAGS} tags.`;
  }

  const long = tags.find((tag) => tag.length > MAX_TAG_LENGTH);

  if (long !== undefined) {
    return `Keep each tag to ${MAX_TAG_LENGTH} characters.`;
  }

  const bad = tags.find((tag) => !TAG_GRAMMAR.test(tag));

  return bad === undefined
    ? undefined
    : `“${bad}” can only use letters, numbers and dashes, starting with a letter or number.`;
}

/** "1 linked object", "3 linked objects". */
export function linkedLabel(count: number): string {
  return `${count} linked ${count === 1 ? "object" : "objects"}`;
}

const digestDay = new Intl.DateTimeFormat(undefined, {
  month: "long",
  day: "numeric",
  year: "numeric",
  timeZone: "UTC",
});

/** "October 7, 2026" from the digest's `YYYY-MM-DD` (a UTC day). */
export function digestDate(date: string): string {
  const millis = Date.parse(`${date}T00:00:00Z`);

  return Number.isNaN(millis) ? date : digestDay.format(millis);
}

/** Only an `https://` run URL goes into an `href`. */
export function safeHttpsUrl(url: string | null): string | null {
  return url?.startsWith("https://") === true ? url : null;
}
