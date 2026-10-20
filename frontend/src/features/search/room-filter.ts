/**
 * The room header's search starts scoped to its room: focusing the empty field fills in a
 * readable filter for the room, plain text the person can delete for a global search.
 *
 * The field shows `in:#launch-planning` (the handle the typeahead writes for a channel), a DM as
 * `in:@ada-lovelace` (its person, as the filter bar names it) and your notes as `in:@me`. The
 * server reads `in:<name>` as a substring of room names (`in:#launch` also matches
 * `#launch-planning`, and a DM or your notes have no channel name), so on submit that token, if
 * still there untouched, becomes `in_id:<room id>`: the one filter naming exactly this room. The
 * search page's filter bar shows it by the room's name ("In: launch-planning").
 */
import type { RoomKind } from "../../store/model.ts";
import { inHandle } from "./typeahead.ts";

/** What the header knows about its room to name it in a filter. */
export interface FilterRoom {
  readonly kind: RoomKind;
  /** The channel's name, or a DM's (its people). */
  readonly name: string;
  readonly noteToSelf: boolean;
}

/** The exact filter for the room `roomId`, as the server receives it. */
export function roomIdToken(roomId: number): string {
  return `in_id:${roomId}`;
}

/**
 * The filter the field shows for `room`: `in:#launch-planning`, `in:@ada-lovelace`, `in:@me`;
 * `null` for a room with no name to show yet.
 */
export function roomFilterToken(room: FilterRoom): string | null {
  if (room.noteToSelf) {
    return "in:@me";
  }

  const handle = inHandle(room.name.trim());

  return handle === "" ? null : `in:${room.kind === "direct" ? "@" : "#"}${handle}`;
}

/** What the header field holds on focus: the room's filter and a space when empty, else as is. */
export function prefillOnFocus(value: string, token: string | null): string {
  return value === "" && token !== null ? `${token} ` : value;
}

/**
 * What the field keeps on blur: nothing when it holds only the untouched `prefill` (so the
 * collapsed field doesn't show a stale filter), else what the person typed.
 */
export function settleOnBlur(value: string, prefill: string | null): string {
  return prefill !== null && value.trim() === prefill.trim() ? "" : value;
}

/**
 * `query` as submitted: each whole, unedited `token` (the filter the field filled in) becomes the
 * room's exact `in_id:` filter. An edited or deleted token leaves what the person typed alone.
 */
export function scopeQuery(query: string, token: string | null, roomId: number): string {
  if (token === null) {
    return query;
  }

  return query
    .split(/(\s+)/u)
    .map((part) => (part === token ? roomIdToken(roomId) : part))
    .join("");
}
