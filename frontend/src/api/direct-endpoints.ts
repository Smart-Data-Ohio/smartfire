/** The S2 direct-message endpoints and the quick switcher's data. */
import { Effect } from "effect";
import type { DirectCandidateList } from "../gen/DirectCandidateList.ts";
import type { RoomDetail } from "../gen/RoomDetail.ts";
import type { SidebarRow } from "../gen/SidebarRow.ts";
import type { Switcher } from "../gen/Switcher.ts";
import { call, get } from "./call.ts";
import { DirectCandidateList as DirectCandidateListSchema } from "./schema/direct.ts";
import { RoomDetail as RoomDetailSchema } from "./schema/room.ts";
import { SidebarRow as SidebarRowSchema } from "./schema/sidebar.ts";
import { Switcher as SwitcherSchema } from "./schema/switcher.ts";
import { wire } from "./wire.ts";

/** `GET /directs/candidates`: everyone the viewer can message. */
export const directCandidates = Effect.fn("api.directCandidates")(function* () {
  return yield* call(
    get("/directs/candidates"),
    wire<DirectCandidateList>(DirectCandidateListSchema),
  );
});

/** `POST /directs`: opens (or finds) the DM with exactly these people. */
export const createDirect = Effect.fn("api.createDirect")(function* (userIds: readonly number[]) {
  return yield* call(
    { method: "POST", path: "/directs", body: { userIds: [...userIds] } },
    wire<SidebarRow>(SidebarRowSchema),
  );
});

/** `POST /directs/:id/members`: adds people to a DM (it becomes a group DM). */
export const addDirectMembers = Effect.fn("api.addDirectMembers")(function* (
  roomId: number,
  userIds: readonly number[],
) {
  return yield* call(
    { method: "POST", path: `/directs/${roomId}/members`, body: { userIds: [...userIds] } },
    wire<RoomDetail>(RoomDetailSchema),
  );
});

/** `PATCH /directs/:id`: names a group DM; `null` goes back to the member list. */
export const renameDirect = Effect.fn("api.renameDirect")(function* (
  roomId: number,
  name: string | null,
) {
  return yield* call(
    { method: "PATCH", path: `/directs/${roomId}`, body: { name } },
    wire<RoomDetail>(RoomDetailSchema),
  );
});

/** `GET /switcher`: rooms, people and recent threads; the client ranks. */
export const switcher = Effect.fn("api.switcher")(function* () {
  return yield* call(get("/switcher"), wire<Switcher>(SwitcherSchema));
});
