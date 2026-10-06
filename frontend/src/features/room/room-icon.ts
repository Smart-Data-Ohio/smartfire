import type { RoomKind } from "../../store/model.ts";
import type { IconName } from "../../ui/icons/icon.tsx";

/** The glyph for each kind of room, wherever a room is named (sidebar, header, intro). */
export const ROOM_KIND_ICON = {
  open: "hash",
  closed: "lock",
  direct: "dms",
  voice: "volume",
  stage: "volume",
  board: "boards",
} as const satisfies Record<RoomKind, IconName>;
