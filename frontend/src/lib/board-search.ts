import type { BoardStatusFilter } from "../gen/BoardStatusFilter.ts";

interface RawBoardSearch {
  readonly view?: unknown;
  readonly status?: unknown;
  readonly owner?: unknown;
  readonly tag?: unknown;
}

export interface BoardSearch {
  readonly view?: "list" | "board";
  readonly status?: BoardStatusFilter;
  readonly owner?: string;
  readonly tag?: string;
}

type MutableBoardSearch = { -readonly [Key in keyof BoardSearch]: BoardSearch[Key] };

/** Room search is inherited by thread, permalink and dialog routes. */
export function parseBoardSearch(search: RawBoardSearch): BoardSearch {
  const parsed: MutableBoardSearch = {};

  if (search.view === "list" || search.view === "board") parsed.view = search.view;

  if (search.status === "open" || search.status === "done" || search.status === "all")
    parsed.status = search.status;

  // TanStack decodes numeric query strings as numbers, including classic owner=7 links.
  if (
    String(search.owner) === search.owner ||
    (Number.isSafeInteger(search.owner) && Number(search.owner) > 0)
  )
    parsed.owner = String(search.owner);

  if (String(search.tag) === search.tag) parsed.tag = String(search.tag);

  return parsed;
}

export interface RoomSearch extends BoardSearch {
  /** A phone's full-screen call view, open over the room (call-view-cover.ts). */
  readonly call?: 1;
}

/** The room's search: the board's, plus `call=1` (which `parseBoardSearch` navigations drop). */
export function parseRoomSearch(search: RawBoardSearch & { readonly call?: unknown }): RoomSearch {
  const parsed = parseBoardSearch(search);

  return search.call === 1 || search.call === "1" ? { ...parsed, call: 1 } : parsed;
}
