/**
 * What the Members and Files panes call: plain promises over their endpoints, with the people
 * each answer names merged into the store. Failures reject with an `ActionError` fit to show.
 */
import { type FileQuery, files, members, setStarred } from "../api/pane-endpoints.ts";
import type { FileList } from "../gen/FileList.ts";
import type { MemberList } from "../gen/MemberList.ts";
import type { StarState } from "../gen/StarState.ts";
import { mutations } from "../store/store.ts";
import { onRoomRefresh } from "./room-refresh.ts";
import { runAction } from "./runtime.ts";
import { onSyncEvents } from "./signals.ts";

export type { FileQuery };

export const panes = {
  /** Mounted member readers reload only after a room-management change. */
  onRoomRefresh,
  /** File readers must refresh even when the edited message is outside a loaded timeline. */
  onMessageUpdate: (roomId: number, listener: () => void): (() => void) =>
    onSyncEvents((events) => {
      if (
        events.some((event) => event.type === "message.updated" && event.data.roomId === roomId)
      ) {
        listener();
      }
    }),
  /** Every active member with presence and the viewer's stars. */
  members: async (roomId: number): Promise<MemberList> => {
    const list = await runAction(members(roomId));

    mutations.mergeUsers(list.users);

    return list;
  },

  /** Stars or unstars someone for the viewer. */
  setStarred: (userId: number, starred: boolean): Promise<StarState> =>
    runAction(setStarred(userId, starred)),

  /** One page of the room's files, newest first. */
  files: async (roomId: number, query: FileQuery): Promise<FileList> => {
    const list = await runAction(files(roomId, query));

    mutations.mergeUsers(list.users);

    return list;
  },
};
