import { useStore as useZustandStore } from "zustand";
import { useShallow } from "zustand/react/shallow";
import { createStore } from "zustand/vanilla";
import type { ActivityItem } from "../gen/ActivityItem.ts";
import type { ActivityList } from "../gen/ActivityList.ts";
import type { ActivityState } from "../gen/ActivityState.ts";
import type { ActivityTab } from "../gen/ActivityTab.ts";
import type { AgentApproval } from "../gen/AgentApproval.ts";
import type { AgentApprovalPage } from "../gen/AgentApprovalPage.ts";
import type { AgentDirectory } from "../gen/AgentDirectory.ts";
import type { AgentLedgerPage } from "../gen/AgentLedgerPage.ts";
import type { AgentProfile } from "../gen/AgentProfile.ts";
import type { MessageReactions } from "../gen/MessageReactions.ts";
import type { PinState } from "../gen/PinState.ts";
import type { SavedFilter } from "../gen/SavedFilter.ts";
import type { SavedItem } from "../gen/SavedItem.ts";
import type { SavedItemList } from "../gen/SavedItemList.ts";
import type { ScheduledMessage } from "../gen/ScheduledMessage.ts";
import type { ScheduledMessageList } from "../gen/ScheduledMessageList.ts";
import type { ThreadCreated } from "../gen/ThreadCreated.ts";
import type { ThreadDetail } from "../gen/ThreadDetail.ts";
import type { ThreadList } from "../gen/ThreadList.ts";
import type { WorkFacts } from "../gen/WorkFacts.ts";
import type { WorkFilter } from "../gen/WorkFilter.ts";
import type { WorkList } from "../gen/WorkList.ts";
import * as activity from "./activity.ts";
import * as agents from "./agents.ts";
import * as approvals from "./approvals.ts";
import * as ledger from "./ledger.ts";
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
import * as savedList from "./saved-list.ts";
import * as scheduled from "./scheduled.ts";
import { initialState, type State } from "./state.ts";
import * as threads from "./threads.ts";
import * as work from "./work.ts";

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
    apply((state) => work.landWorkDetail(threads.loadThreadDetail(state, detail), detail)),
  /** A thread started here: its pane data, and the first reply on its (new) timeline. */
  threadCreated: (created: ThreadCreated) =>
    apply((state) =>
      reduce.receiveMessage(
        work.landWorkDetail(threads.loadThreadDetail(state, created.detail), created.detail),
        created.message,
      ),
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
  // --- S3: the activity inbox, saved items and scheduled messages ---
  setActivityListLoading: (tab: ActivityTab, status: ActivityState, more: boolean) =>
    apply((state) => activity.setActivityListLoading(state, tab, status, more)),
  setActivityListFailed: (
    tab: ActivityTab,
    status: ActivityState,
    error: string,
    generation?: number,
  ) => apply((state) => activity.setActivityListFailed(state, tab, status, error, generation)),
  landActivityPage: (
    tab: ActivityTab,
    status: ActivityState,
    page: ActivityList,
    mode: "replace" | "more",
    start?: activity.ActivityLoadStart,
  ) => apply((state) => activity.landActivityPage(state, tab, status, page, mode, start)),
  /** An item as the server has it now; `unreadCount` `null` keeps the badge. */
  applyActivityItem: (item: ActivityItem, unreadCount: number | null) =>
    apply((state) => activity.applyActivityItem(state, item, unreadCount)),
  /** A change on its way, shown at once (see `activity.showActivityChange`). */
  showActivityChange: (item: ActivityItem, token: number, unreadDelta: number) =>
    apply((state) => activity.showActivityChange(state, item, token, unreadDelta)),
  /** A change on its way ended (see `activity.endActivityChange`). */
  endActivityChange: (end: activity.ActivityChangeEnd) =>
    apply((state) => activity.endActivityChange(state, end)),
  setActivityUnreadCount: (unreadCount: number) =>
    apply((state) => activity.setActivityUnreadCount(state, unreadCount)),
  setSavedListLoading: (filter: SavedFilter, more: boolean) =>
    apply((state) => savedList.setSavedListLoading(state, filter, more)),
  setSavedListFailed: (filter: SavedFilter, error: string, generation?: number) =>
    apply((state) => savedList.setSavedListFailed(state, filter, error, generation)),
  landSavedPage: (
    filter: SavedFilter,
    page: SavedItemList,
    mode: "replace" | "more",
    generation?: number,
  ) => apply((state) => savedList.landSavedPage(state, filter, page, mode, generation)),
  /** A message saved, changed (`item`) or unsaved (`null`): its mark, item and lists follow. */
  applySavedChange: (messageId: number, item: SavedItem | null) =>
    apply((state) => savedList.applySavedChange(state, messageId, item)),
  setScheduledListLoading: (key: scheduled.ScheduledListKey, more: boolean) =>
    apply((state) => scheduled.setScheduledListLoading(state, key, more)),
  setScheduledListFailed: (key: scheduled.ScheduledListKey, error: string, generation?: number) =>
    apply((state) => scheduled.setScheduledListFailed(state, key, error, generation)),
  landScheduledPage: (
    key: scheduled.ScheduledListKey,
    page: ScheduledMessageList,
    mode: "replace" | "more",
    generation?: number,
  ) => apply((state) => scheduled.landScheduledPage(state, key, page, mode, generation)),
  applyScheduled: (message: ScheduledMessage) =>
    apply((state) => scheduled.applyScheduled(state, message)),
  removeScheduled: (id: number) => apply((state) => scheduled.removeScheduled(state, id)),
  /** Every saved list reloads when next shown (a change the server says is already gone). */
  markSavedStale: () => apply((state) => savedList.markSavedStale(state)),
  /** Every scheduled list reloads when next shown (a send that dropped it instead). */
  markScheduledStale: () => apply((state) => scheduled.markScheduledStale(state)),
  /**
   * Missed events the server can't replay: every S3 list, and every agent's approvals, reload when
   * next shown.
   */
  markInboxStale: () =>
    apply((state) =>
      approvals.markApprovalsStale(
        scheduled.markScheduledStale(savedList.markSavedStale(activity.markActivityStale(state))),
      ),
    ),
  // --- S4: work tracking ---
  /** Shows work facts on a thread at once: an optimistic change or its rollback. */
  putWorkFacts: (threadId: number, facts: WorkFacts | null) =>
    apply((state) => work.putWorkFacts(state, threadId, facts)),
  countWorkWrite: (threadId: number, delta: 1 | -1) =>
    apply((state) => work.countWorkWrite(state, threadId, delta)),
  setWorkListLoading: (filter: WorkFilter) =>
    apply((state) => work.setWorkListLoading(state, filter)),
  setWorkListFailed: (filter: WorkFilter, error: string, generation: number) =>
    apply((state) => work.setWorkListFailed(state, filter, error, generation)),
  landWorkList: (filter: WorkFilter, list: WorkList, generation: number) =>
    apply((state) => work.landWorkList(state, filter, list, generation)),
  // --- S4: agents ---
  setAgentDirectoryLoading: () => apply((state) => agents.setDirectoryLoading(state)),
  landAgentDirectory: (page: AgentDirectory, generation: number) =>
    apply((state) => agents.landDirectory(state, page, generation)),
  setAgentDirectoryFailed: (error: string, generation: number) =>
    apply((state) => agents.setDirectoryFailed(state, error, generation)),
  setAgentProfileLoading: (agentId: number) =>
    apply((state) => agents.setProfileLoading(state, agentId)),
  landAgentProfile: (profile: AgentProfile) => apply((state) => agents.landProfile(state, profile)),
  setAgentProfileFailed: (agentId: number, error: string, missing: boolean) =>
    apply((state) => agents.setProfileFailed(state, agentId, error, missing)),
  setApprovalListLoading: (key: approvals.ApprovalListKey, more: boolean) =>
    apply((state) => approvals.setApprovalListLoading(state, key, more)),
  setApprovalListFailed: (key: approvals.ApprovalListKey, error: string, generation?: number) =>
    apply((state) => approvals.setApprovalListFailed(state, key, error, generation)),
  landApprovalPage: (
    key: approvals.ApprovalListKey,
    page: AgentApprovalPage,
    mode: "replace" | "more",
    generation?: number,
  ) => apply((state) => approvals.landApprovalPage(state, key, page, mode, generation)),
  applyApproval: (approval: AgentApproval) =>
    apply((state) => approvals.applyApproval(state, approval)),
  /** Every approvals list reloads when next shown. */
  markApprovalsStale: () => apply((state) => approvals.markApprovalsStale(state)),
  setLedgerListLoading: (key: ledger.LedgerListKey, more: boolean) =>
    apply((state) => ledger.setLedgerListLoading(state, key, more)),
  setLedgerListFailed: (
    key: ledger.LedgerListKey,
    agentId: number,
    error: string,
    forbidden: boolean,
    generation?: number,
  ) =>
    apply((state) => ledger.setLedgerListFailed(state, key, agentId, error, forbidden, generation)),
  landLedgerPage: (
    key: ledger.LedgerListKey,
    agentId: number,
    page: AgentLedgerPage,
    mode: "replace" | "more",
    generation?: number,
  ) => apply((state) => ledger.landLedgerPage(state, key, agentId, page, mode, generation)),
  /** Back to an empty store (tests). */
  reset: () => apply(() => initialState),
};

export type Mutations = typeof mutations;
