import { Clock, Effect } from "effect";
import * as api from "../api/room-endpoints.ts";
import type { CreateRoom } from "../gen/CreateRoom.ts";
import type { RoomForm } from "../gen/RoomForm.ts";
import type { RoomKind } from "../gen/RoomKind.ts";
import type { RoomMutation } from "../gen/RoomMutation.ts";
import type { UpdateRoom } from "../gen/UpdateRoom.ts";
import { mutations, store } from "../store/store.ts";
import { invalidateRoom } from "./room-refresh.ts";

const landForm = (form: RoomForm): RoomForm => {
  mutations.mergeUsers(form.users);

  return form;
};

const landMutation = Effect.fn("rooms.landMutation")(function* (result: RoomMutation) {
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

export const create = Effect.fn("rooms.create")(function* (body: CreateRoom) {
  return yield* landMutation(yield* api.createRoom(body));
});

export const update = Effect.fn("rooms.update")(function* (roomId: number, body: UpdateRoom) {
  return yield* landMutation(yield* api.updateRoom(roomId, body));
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
