/**
 * The direct-message and quick-switcher actions as Effect programs: each calls its endpoint and
 * lands the reply in the store (users first, so every name and avatar resolves), so the
 * `sidebar.row.upserted` event that follows is a no-op.
 */
import { Clock, Effect } from "effect";
import * as api from "../api/direct-endpoints.ts";
import { beginRoomRequest } from "../store/join-state.ts";
import type { RoomDetail, SidebarRow } from "../store/model.ts";
import { mutations, store } from "../store/store.ts";
import { invalidateRoom } from "./room-refresh.ts";
import { withRowTicket } from "./row-ticket.ts";

/**
 * Upserts `row` through the same reducer the `sidebar.row.upserted` event uses, unless the sync
 * path changed the room's row after `since` (the request's row ticket).
 */
const upsertRow = Effect.fn("directs.upsertRow")(function* (row: SidebarRow, since: number) {
  const viewerId = store.getState().me?.user.id ?? store.getState().boot?.user.id ?? 0;

  // A local event: `seq` only matters to the sync cursor, which never sees this one.
  mutations.landReplyRows(
    [{ seq: 0, topic: `user:${viewerId}`, type: "sidebar.row.upserted", data: row }],
    yield* Clock.currentTimeMillis,
    since,
  );
});

/** A renamed or grown DM's header data, mirrored onto its sidebar row (label and avatars). */
const landDetail = Effect.fn("directs.landDetail")(function* (detail: RoomDetail, since: number) {
  const started = beginRoomRequest();

  if (!mutations.setRoomDetail(detail, started)) {
    return detail;
  }

  const row = store.getState().sidebar.rows[detail.room.id];

  if (row !== undefined) {
    yield* upsertRow(
      {
        ...row,
        room: detail.room,
        displayName: detail.displayName,
        directMemberIds: detail.directMemberIds,
      },
      since,
    );
  }

  invalidateRoom(detail.room.id);

  return detail;
});

/** Everyone the viewer can message (starred first), with their profiles in the store. */
export const candidates = Effect.fn("directs.candidates")(function* () {
  const list = yield* api.directCandidates();

  mutations.mergeUsers(list.users);

  return list.candidates;
});

/** Opens (or finds) the DM with exactly these people; its sidebar row lands at once. */
export const create = Effect.fn("directs.create")(function* (userIds: readonly number[]) {
  return yield* withRowTicket((since) =>
    Effect.gen(function* () {
      const row = yield* api.createDirect(userIds);

      yield* upsertRow(row, since);

      return row;
    }),
  );
});

/** Adds people to a group-capable DM; the header and the sidebar row update at once. */
export const addMembers = Effect.fn("directs.addMembers")(function* (
  roomId: number,
  userIds: readonly number[],
) {
  return yield* withRowTicket((since) =>
    api
      .addDirectMembers(roomId, userIds)
      .pipe(Effect.flatMap((detail) => landDetail(detail, since))),
  );
});

/** Names a group DM (blank or `null` goes back to the members' names). */
export const rename = Effect.fn("directs.rename")(function* (roomId: number, name: string | null) {
  const trimmed = name?.trim() ?? "";

  return yield* withRowTicket((since) =>
    api
      .renameDirect(roomId, trimmed === "" ? null : trimmed)
      .pipe(Effect.flatMap((detail) => landDetail(detail, since))),
  );
});

/** The quick switcher's catalogue: rooms, people and recent threads, profiles in the store. */
export const switcherData = Effect.fn("directs.switcherData")(function* () {
  const data = yield* api.switcher();

  mutations.mergeUsers(data.users);

  return data;
});
