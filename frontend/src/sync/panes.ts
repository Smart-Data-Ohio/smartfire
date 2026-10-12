/**
 * What the Members and Files panes call: plain promises over their endpoints, with the people
 * each answer names merged into the store. Failures reject with an `ActionError` fit to show.
 */
import { type FileQuery, files, members, setStarred } from "../api/pane-endpoints.ts";
import type { FileList } from "../gen/FileList.ts";
import type { MemberList } from "../gen/MemberList.ts";
import type { MessageDTO } from "../gen/MessageDTO.ts";
import type { StarState } from "../gen/StarState.ts";
import { mutations } from "../store/store.ts";
import { onRoomRefresh } from "./room-refresh.ts";
import { runAction } from "./runtime.ts";
import { onResync, onSyncEvents } from "./signals.ts";

export type { FileQuery };

export const panes = {
  /** Mounted member readers reload only after a room-management change. */
  onRoomRefresh,
  /**
   * A room message posted or changed, with its complete attachment list. Includes posts and
   * edits outside loaded timelines; `created` tells a fresh post from an edit.
   */
  onMessageChange: (
    roomId: number,
    listener: (message: MessageDTO, created: boolean) => void,
  ): (() => void) =>
    onSyncEvents((events) => {
      for (const event of events) {
        if (
          (event.type === "message.created" || event.type === "message.updated") &&
          event.data.roomId === roomId
        ) {
          listener(event.data, event.type === "message.created");
        }
      }
    }),
  /** A room message deleted, by id. Its files go with it. */
  onMessageRemoved: (roomId: number, listener: (messageId: number) => void): (() => void) =>
    onSyncEvents((events) => {
      for (const event of events) {
        if (event.type === "message.removed" && event.data.roomId === roomId) {
          listener(event.data.id);
        }
      }
    }),
  /** A reply changed files without publishing its message on the room topic. */
  onFilesChanged: (roomId: number, listener: () => void): (() => void) => {
    const stopEvents = onSyncEvents((events) => {
      if (
        events.some((event) => event.type === "room.files.changed" && event.data.roomId === roomId)
      ) {
        listener();
      }
    });

    const stopResync = onResync((topics) => {
      if (topics.includes("user") || topics.includes(`room:${roomId}`)) listener();
    });

    return () => {
      stopEvents();
      stopResync();
    };
  },
  onThreadRemoved: (roomId: number, listener: (threadId: number) => void): (() => void) =>
    onSyncEvents((events) => {
      for (const event of events) {
        if (event.type === "thread.removed" && event.data.roomId === roomId) {
          listener(event.data.threadId);
        }
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
