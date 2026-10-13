import type { SidebarRow } from "./model.ts";

/** What a room needs for the test: its kind and the members the server lists for a DM. */
export type DirectSource = Pick<SidebarRow, "directMemberIds"> & {
  readonly room: Pick<SidebarRow["room"], "kind">;
};

/**
 * Whether this is the viewer's note-to-self (Slack's "message yourself"): a DM whose only member
 * is them, which the server lists as `directMemberIds: [viewer]`.
 */
export function isNoteToSelf(source: DirectSource, viewerId: number | null): boolean {
  return (
    viewerId !== null &&
    source.room.kind === "direct" &&
    source.directMemberIds.length === 1 &&
    source.directMemberIds[0] === viewerId
  );
}
