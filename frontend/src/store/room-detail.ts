import type { RoomDetail, SidebarRow } from "./model.ts";

export function withSidebarRow(detail: RoomDetail, row: SidebarRow | undefined): RoomDetail {
  if (row === undefined) {
    return detail;
  }

  return {
    ...detail,
    room: row.room,
    membership: row.membership,
    displayName: row.room.kind === "direct" ? detail.displayName : row.displayName,
    directMemberIds: row.directMemberIds,
  };
}
