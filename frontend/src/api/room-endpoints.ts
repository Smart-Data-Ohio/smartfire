import { Effect } from "effect";
import type { CreateRoom } from "../gen/CreateRoom.ts";
import type { RoomForm } from "../gen/RoomForm.ts";
import type { RoomKind } from "../gen/RoomKind.ts";
import type { RoomLeft } from "../gen/RoomLeft.ts";
import type { RoomMutation } from "../gen/RoomMutation.ts";
import type { RoomRemoved } from "../gen/RoomRemoved.ts";
import type { UpdateRoom } from "../gen/UpdateRoom.ts";
import { call, get } from "./call.ts";
import {
  RoomForm as RoomFormSchema,
  RoomLeft as RoomLeftSchema,
  RoomMutation as RoomMutationSchema,
  RoomRemoved as RoomRemovedSchema,
} from "./schema/room-management.ts";
import { wire } from "./wire.ts";

export const newRoom = Effect.fn("api.newRoom")(function* (type: RoomKind) {
  return yield* call(get("/rooms/new", { type }), wire<RoomForm>(RoomFormSchema));
});

export const editRoom = Effect.fn("api.editRoom")(function* (roomId: number) {
  return yield* call(get(`/rooms/${roomId}/edit`), wire<RoomForm>(RoomFormSchema));
});

/** A retry must reuse this request's clientRoomId. */
export const createRoom = Effect.fn("api.createRoom")(function* (body: CreateRoom) {
  return yield* call(
    { method: "POST", path: "/rooms", body },
    wire<RoomMutation>(RoomMutationSchema),
  );
});

export const updateRoom = Effect.fn("api.updateRoom")(function* (roomId: number, body: UpdateRoom) {
  return yield* call(
    { method: "PATCH", path: `/rooms/${roomId}`, body },
    wire<RoomMutation>(RoomMutationSchema),
  );
});

export const removeRoom = Effect.fn("api.removeRoom")(function* (roomId: number) {
  return yield* call(
    { method: "DELETE", path: `/rooms/${roomId}` },
    wire<RoomRemoved>(RoomRemovedSchema),
  );
});

/** The leave action offered by a direct room's edit page. */
export const leaveDirectRoom = Effect.fn("api.leaveDirectRoom")(function* (roomId: number) {
  return yield* call(
    { method: "DELETE", path: `/rooms/${roomId}/membership` },
    wire<RoomLeft>(RoomLeftSchema),
  );
});
