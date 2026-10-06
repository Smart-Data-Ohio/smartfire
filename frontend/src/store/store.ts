import { useStore as useZustandStore } from "zustand";
import { useShallow } from "zustand/react/shallow";
import { createStore } from "zustand/vanilla";
import type { MessageReactions } from "../gen/MessageReactions.ts";
import type { PinState } from "../gen/PinState.ts";
import type { ThreadCreated } from "../gen/ThreadCreated.ts";
import type { ThreadDetail } from "../gen/ThreadDetail.ts";
import type { ThreadList } from "../gen/ThreadList.ts";
import * as extras from "./message-extras.ts";
import type {
  Boot,
  ConnectionStatus,
  Me,
  MessageDTO,
  MessagePage,
  PendingMessage,
  RoomDetail,
  Sidebar,
  SyncEvent,
  Thread,
  ThreadFilter,
  ThreadMembership,
  User,
  UserPresence,
} from "./model.ts";
import * as reduce from "./reducers.ts";
import { initialState, type State } from "./state.ts";
import * as threads from "./threads.ts";

/**
 * The live store. Plain TypeScript, no Effect: the sync engine (src/sync) writes it through
 * `mutations`, components read it through `useStore(selector)`.
 */
export const store = createStore<State>()(() => initialState);

export function useStore<T>(selector: (state: State) => T): T {
  return useZustandStore(store, selector);
}

/** The messages among `ids` the store holds, as a record that only changes when one of them does. */
export function useMessagesIn(ids: readonly number[]): Readonly<Record<number, MessageDTO>> {
  return useZustandStore(
    store,
    useShallow((state: State) => {
      const picked: Record<number, MessageDTO> = {};

      for (const id of ids) {
        const message = state.messages[id];

        if (message !== undefined) {
          picked[id] = message;
        }
      }

      return picked;
    }),
  );
}

const apply = (change: (state: State) => State) => store.setState(change, true);

/** Every write to the store. Each is one `setState`, so one React commit. */
export const mutations = {
  setBoot: (boot: Boot) => apply((state) => ({ ...state, boot })),
  setMe: (me: Me) => apply((state) => reduce.setMe(state, me)),
  setConnection: (connection: ConnectionStatus) =>
    apply((state) => (state.connection === connection ? state : { ...state, connection })),
  mergeUsers: (users: readonly User[]) => apply((state) => reduce.mergeUsers(state, users)),
  setPresence: (list: readonly UserPresence[]) => apply((state) => reduce.setPresence(state, list)),
  setSidebarLoading: () =>
    apply((state) => ({ ...state, sidebar: { ...state.sidebar, status: "loading" } })),
  setSidebarFailed: () =>
    apply((state) => ({ ...state, sidebar: { ...state.sidebar, status: "error" } })),
  loadSidebar: (sidebar: Sidebar) => apply((state) => reduce.loadSidebar(state, sidebar)),
  markRoomRead: (roomId: number) => apply((state) => reduce.markRoomRead(state, roomId)),
  setRoomLoading: (roomId: number) => apply((state) => reduce.setRoomLoading(state, roomId)),
  setRoomError: (roomId: number, error: string) =>
    apply((state) => reduce.setRoomError(state, roomId, error)),
  setRoomDetail: (detail: RoomDetail) => apply((state) => reduce.setRoomDetail(state, detail)),
  applyPage: (roomId: number, page: MessagePage, mode: reduce.PageMode) =>
    apply((state) => reduce.applyPage(state, roomId, page, mode)),
  setPageLoading: (roomId: number, direction: "older" | "newer") =>
    apply((state) => reduce.setPageLoading(state, roomId, direction)),
  setPageFailed: (roomId: number) => apply((state) => reduce.setPageFailed(state, roomId)),
  setPageReplacing: (roomId: number) => apply((state) => reduce.setPageReplacing(state, roomId)),
  clearUnreadDivider: (roomId: number) =>
    apply((state) => reduce.clearUnreadDivider(state, roomId)),
  moveUnreadDivider: (roomId: number, fromId: number) =>
    apply((state) => reduce.moveUnreadDivider(state, roomId, fromId)),
  markUnreadFrom: (roomId: number, fromId: number, now: number) =>
    apply((state) => reduce.markUnreadFrom(state, roomId, fromId, now)),
  receiveMessage: (message: MessageDTO) => apply((state) => reduce.receiveMessage(state, message)),
  addPending: (pending: PendingMessage) => apply((state) => reduce.addPending(state, pending)),
  setPendingState: (
    clientMessageId: string,
    status: PendingMessage["state"],
    error: string | null,
  ) => apply((state) => reduce.setPendingState(state, clientMessageId, status, error)),
  discardPending: (clientMessageId: string) =>
    apply((state) => reduce.discardPending(state, clientMessageId)),
  applyEvents: (events: readonly SyncEvent[], now: number) =>
    apply((state) => (events.length === 0 ? state : reduce.applyEvents(state, events, now))),
  prune: (now: number) => apply((state) => reduce.prune(state, now)),
  /** An edit's reply (the `message.updated` event may beat it; the newer copy wins). */
  updateMessage: (message: MessageDTO) => apply((state) => reduce.updateMessage(state, message)),
  /** A delete went through here; the event may follow (or have come first). */
  removeMessage: (message: MessageDTO, now: number) =>
    apply((state) =>
      reduce.removeMessage(state, message.id, message.roomId, message.threadId, now),
    ),
  setReactions: (change: MessageReactions) => apply((state) => extras.setReactions(state, change)),
  setPinState: (change: PinState) => apply((state) => extras.setPinState(state, change)),
  setSavedMark: (messageId: number, savedItemId: number | null) =>
    apply((state) => extras.setSavedMark(state, messageId, savedItemId)),
  applyThreadPage: (threadId: number, page: MessagePage, mode: reduce.PageMode) =>
    apply((state) => reduce.applyThreadPage(state, threadId, page, mode)),
  setThreadPageLoading: (threadId: number, direction: "older" | "newer") =>
    apply((state) => reduce.setThreadPageLoading(state, threadId, direction)),
  setThreadPageFailed: (threadId: number) =>
    apply((state) => reduce.setThreadPageFailed(state, threadId)),
  setThreadPageReplacing: (threadId: number) =>
    apply((state) => reduce.setThreadPageReplacing(state, threadId)),
  setThreadPaneLoading: (threadId: number) =>
    apply((state) => threads.setThreadPaneLoading(state, threadId)),
  setThreadPaneError: (threadId: number, error: string) =>
    apply((state) => threads.setThreadPaneError(state, threadId, error)),
  loadThreadDetail: (detail: ThreadDetail) =>
    apply((state) => threads.loadThreadDetail(state, detail)),
  /** A thread started here: its pane data, and the first reply on its (new) timeline. */
  threadCreated: (created: ThreadCreated) =>
    apply((state) =>
      reduce.receiveMessage(threads.loadThreadDetail(state, created.detail), created.message),
    ),
  upsertThread: (thread: Thread) => apply((state) => threads.upsertThread(state, thread)),
  setThreadMembership: (threadId: number, membership: ThreadMembership | null) =>
    apply((state) => threads.setThreadMembership(state, threadId, membership)),
  setThreadListLoading: (roomId: number, filter: ThreadFilter) =>
    apply((state) => threads.setThreadListLoading(state, roomId, filter)),
  setThreadListFailed: (roomId: number, filter: ThreadFilter) =>
    apply((state) => threads.setThreadListFailed(state, roomId, filter)),
  loadThreadList: (roomId: number, filter: ThreadFilter, list: ThreadList) =>
    apply((state) => threads.loadThreadList(state, roomId, filter, list)),
  /** Back to an empty store (tests). */
  reset: () => apply(() => initialState),
};

export type Mutations = typeof mutations;
