import { Clock, Effect, Predicate, Result } from "effect";
import { room } from "../api/endpoints.ts";
import * as api from "../api/room-endpoints.ts";
import type { CreateRoom } from "../gen/CreateRoom.ts";
import type { RoomForm } from "../gen/RoomForm.ts";
import type { RoomKind } from "../gen/RoomKind.ts";
import type { RoomMutation } from "../gen/RoomMutation.ts";
import type { UpdateRoom } from "../gen/UpdateRoom.ts";
import { mutations, store } from "../store/store.ts";
import { changedSince, invalidateRoom, managementEpoch, roomRevision } from "./room-refresh.ts";

const landForm = (form: RoomForm): RoomForm => {
  mutations.mergeUsers(form.users);

  return form;
};

/**
 * A room the store learned about after a write began: the write's reply is older than the store, so
 * it doesn't land. The room is read again instead: its detail (with the newer sidebar row's facts),
 * or unavailable if the viewer lost it (say another admin removed them meanwhile).
 */
const refetchSuperseded = Effect.fn("rooms.refetchSuperseded")(function* (roomId: number) {
  const revision = invalidateRoom(roomId);
  const fetched = yield* Effect.result(room(roomId));

  if (roomRevision(roomId) !== revision) {
    return;
  }

  if (Result.isFailure(fetched)) {
    if (Predicate.isTagged(fetched.failure, "NotFound")) {
      mutations.setRoomUnavailable(roomId);
    }

    return;
  }

  const row = store.getState().sidebar.rows[roomId];

  mutations.setRoomDetail(
    row === undefined
      ? fetched.success
      : {
          ...fetched.success,
          room: row.room,
          membership: row.membership,
          displayName: row.displayName,
          directMemberIds: row.directMemberIds,
        },
  );
});

/**
 * Lands a create/update reply, unless a management change to the room reached the store after the
 * write began (`since`, a `managementEpoch()`): then the reply could bring back what that change
 * took away, so the room is read again instead.
 */
const landMutation = Effect.fn("rooms.landMutation")(function* (
  result: RoomMutation,
  since: number,
) {
  if (changedSince(result.room.id, since)) {
    yield* refetchSuperseded(result.room.id);

    return result;
  }

  if (result.detail === null) {
    mutations.setRoomUnavailable(result.room.id);
  } else {
    mutations.setRoomDetail(result.detail);
  }

  const viewerId = store.getState().me?.user.id ?? store.getState().boot?.user.id ?? 0;

  mutations.applyEvents(
    result.row === null
      ? [
          {
            seq: 0,
            topic: `user:${viewerId}`,
            type: "sidebar.row.removed",
            data: { roomId: result.room.id },
          },
        ]
      : [{ seq: 0, topic: `user:${viewerId}`, type: "sidebar.row.upserted", data: result.row }],
    yield* Clock.currentTimeMillis,
  );
  invalidateRoom(result.room.id);

  return result;
});

export const newForm = Effect.fn("rooms.newForm")(function* (type: RoomKind) {
  return landForm(yield* api.newRoom(type));
});

export const editForm = Effect.fn("rooms.editForm")(function* (roomId: number) {
  return landForm(yield* api.editRoom(roomId));
});

/** `body.clientRoomId` names the attempt: sending it again returns the room it already made. */
export const create = Effect.fn("rooms.create")(function* (body: CreateRoom) {
  const since = managementEpoch();

  return yield* landMutation(yield* api.createRoom(body), since);
});

export const update = Effect.fn("rooms.update")(function* (roomId: number, body: UpdateRoom) {
  const since = managementEpoch();

  return yield* landMutation(yield* api.updateRoom(roomId, body), since);
});

export const remove = Effect.fn("rooms.remove")(function* (roomId: number) {
  const result = yield* api.removeRoom(roomId);

  mutations.setRoomUnavailable(result.roomId);
  invalidateRoom(result.roomId);

  return result;
});

export const leaveDirect = Effect.fn("rooms.leaveDirect")(function* (roomId: number) {
  const result = yield* api.leaveDirectRoom(roomId);

  mutations.setRoomUnavailable(result.roomId);
  invalidateRoom(result.roomId);

  return result;
});
