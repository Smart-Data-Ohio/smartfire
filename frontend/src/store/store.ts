import { useStore as useZustandStore } from "zustand";
import { createStore } from "zustand/vanilla";
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
  User,
  UserPresence,
} from "./model.ts";
import * as reduce from "./reducers.ts";
import { initialState, type State } from "./state.ts";

/**
 * The live store. Plain TypeScript, no Effect: the sync engine (src/sync) writes it through
 * `mutations`, components read it through `useStore(selector)`.
 */
export const store = createStore<State>()(() => initialState);

export function useStore<T>(selector: (state: State) => T): T {
  return useZustandStore(store, selector);
}

const apply = (change: (state: State) => State) => store.setState(change, true);

/** Every write to the store. Each is one `setState`, so one React commit. */
export const mutations = {
  setBoot: (boot: Boot) => apply((state) => ({ ...state, boot })),
  setMe: (me: Me) => apply((state) => reduce.setMe(state, me)),
  setConnection: (connection: ConnectionStatus) =>
    apply((state) => (state.connection === connection ? state : { ...state, connection })),
  mergeUsers: (users: readonly User[]) => apply((state) => reduce.mergeUsers(state, users)),
  setPresence: (list: readonly UserPresence[]) =>
    apply((state) => reduce.setPresence(state, list)),
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
  clearUnreadDivider: (roomId: number) =>
    apply((state) => reduce.clearUnreadDivider(state, roomId)),
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
  /** Back to an empty store (tests). */
  reset: () => apply(() => initialState),
};

export type Mutations = typeof mutations;
