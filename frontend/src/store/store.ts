import { useStore as useZustandStore } from "zustand";
import { useShallow } from "zustand/react/shallow";
import { createStore } from "zustand/vanilla";
import type { ActivityItem } from "../gen/ActivityItem.ts";
import type { ActivityList } from "../gen/ActivityList.ts";
import type { ActivityState } from "../gen/ActivityState.ts";
import type { ActivityTab } from "../gen/ActivityTab.ts";
import type { ActivityUnreadCount } from "../gen/ActivityUnreadCount.ts";
import type { AgentApproval } from "../gen/AgentApproval.ts";
import type { AgentApprovalPage } from "../gen/AgentApprovalPage.ts";
import type { AgentDirectory } from "../gen/AgentDirectory.ts";
import type { AgentLedgerPage } from "../gen/AgentLedgerPage.ts";
import type { AgentProfile } from "../gen/AgentProfile.ts";
import type { BoardListing } from "../gen/BoardListing.ts";
import type { HuddlePresence } from "../gen/HuddlePresence.ts";
import type { HuddlePresenceList } from "../gen/HuddlePresenceList.ts";
import type { MessageReactions } from "../gen/MessageReactions.ts";
import type { PinState } from "../gen/PinState.ts";
import type { SavedFilter } from "../gen/SavedFilter.ts";
import type { SavedItem } from "../gen/SavedItem.ts";
import type { SavedItemList } from "../gen/SavedItemList.ts";
import type { ScheduledMessage } from "../gen/ScheduledMessage.ts";
import type { ScheduledMessageList } from "../gen/ScheduledMessageList.ts";
import type { StageDetail } from "../gen/StageDetail.ts";
import type { StageState } from "../gen/StageState.ts";
import type { ThreadCreated } from "../gen/ThreadCreated.ts";
import type { ThreadDetail } from "../gen/ThreadDetail.ts";
import type { ThreadList } from "../gen/ThreadList.ts";
import type { WorkFacts } from "../gen/WorkFacts.ts";
import type { WorkFilter } from "../gen/WorkFilter.ts";
import type { WorkList } from "../gen/WorkList.ts";
import type { WorkspaceBranding } from "../gen/WorkspaceBranding.ts";
import * as activity from "./activity.ts";
import * as agents from "./agents.ts";
import * as approvals from "./approvals.ts";
import * as boards from "./boards.ts";
import * as freshness from "./freshness.ts";
import * as huddles from "./huddles.ts";
import {
  beginRoomRequest,
  claimRoomOutcome,
  clearRoomJoin,
  dirtyRoomReread,
} from "./join-state.ts";
import * as ledger from "./ledger.ts";
import * as extras from "./message-extras.ts";
import type {
  Boot,
  ConnectionStatus,
  Me,
  Membership,
  MessageDTO,
  MessagePage,
  OpenRoomPreview,
  PendingMessage,
  RoomCategory,
  RoomDetail,
  Sidebar,
  SidebarRow,
  SyncEvent,
  Thread,
  ThreadFilter,
  ThreadMembership,
  User,
  UserPresence,
} from "./model.ts";
import * as organize from "./organize.ts";
import * as reduce from "./reducers.ts";
import {
  isStale,
  pruneTouches,
  removalIsStale,
  rowClock,
  touchRows,
  untouchedReplyEvents,
} from "./row-touches.ts";
import * as savedList from "./saved-list.ts";
import * as scheduled from "./scheduled.ts";
import { initialState, type State } from "./state.ts";
import * as threads from "./threads.ts";
import * as work from "./work.ts";
import { setWorkspaceBranding, setWorkspaceStyles } from "./workspace.ts";

/**
 * The live store. Plain TypeScript, no Effect: the sync engine (src/sync) writes it through
 * `mutations`, components read it through `useStore(selector)`.
 */
export const store = createStore<State>()(() => initialState);

/**
 * The sidebar row clock now: the ticket a request opening now would hold. Requests take theirs
 * through `withRowTicket` (src/sync/row-ticket.ts), which releases it too.
 */
export const sidebarRowClock = (): number => rowClock(store.getState());

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

/**
 * The row tickets of the HTTP requests in flight, oldest first (several can share a clock). Not
 * state anyone renders, so held outside the store: it only bounds the touches the store keeps.
 */
const heldTickets: number[] = [];

/** Every change ends by forgetting the row touches no request in flight is older than. */
const apply = (change: (state: State) => State) =>
  store.setState(
    (state) => agents.reconcileAgentBadges(pruneTouches(change(state), heldTickets[0])),
    true,
  );

/**
 * Room-read bookkeeping for `sidebar.row.*` events about to land: a removal or a membership change
 * claims the room's outcome, so a read that started earlier must not restore what they changed.
 */
function noteRowEvents(events: readonly SyncEvent[]): void {
  let held = { ...store.getState().sidebar.rows };

  for (const event of events) {
    if (event.type === "sidebar.row.removed") {
      // Access changed at delivery, so a read that started earlier must not restore the room.
      claimRoomOutcome(event.data.roomId, beginRoomRequest());
      clearRoomJoin(event.data.roomId);

      const { [event.data.roomId]: _removed, ...rest } = held;

      held = rest;
      continue;
    }

    if (event.type !== "sidebar.row.upserted") continue;

    const row = event.data;

    if (membershipChanged(held[row.room.id], row)) {
      claimRoomOutcome(row.room.id, beginRoomRequest());
      dirtyRoomReread(row.room.id);
    }

    held[row.room.id] = row;
  }
}

/**
 * A room outcome lands only when `started` is newer than the one already applied.
 * Reads pass the sequence from when they started; confirmed facts pass one taken on arrival.
 */
function landRoom(roomId: number, started: number, change: (state: State) => State): boolean {
  if (!claimRoomOutcome(roomId, started)) return false;
  apply(change);

  return true;
}

function sameMembers(left: readonly number[], right: readonly number[]): boolean {
  if (left.length !== right.length) return false;

  for (let index = 0; index < left.length; index++) {
    if (left[index] !== right[index]) return false;
  }

  return true;
}

/**
 * Involvement, the membership itself, or who a direct row names. Unread, category, favourite
 * order and a rename do not count: those must not retire a room read.
 */
function membershipChanged(previous: SidebarRow | undefined, row: SidebarRow): boolean {
  if (previous === undefined) return true;

  const before = previous.membership;
  const after = row.membership;

  return (
    before.id !== after.id ||
    before.userId !== after.userId ||
    before.involvement !== after.involvement ||
    before.stageRole !== after.stageRole ||
    !sameMembers(previous.directMemberIds, row.directMemberIds)
  );
}

/** Every write to the store. Each is one `setState`, so one React commit. */
export const mutations = {
  setBoardLoading: (roomId: number, query: boards.BoardQuery, more = false) =>
    apply((state) => boards.setBoardLoading(state, roomId, query, more)),
  setBoardError: (roomId: number, generation: number, error: string) =>
    apply((state) => boards.setBoardError(state, roomId, generation, error)),
  loadBoardListing: (listing: BoardListing, generation: number, read?: work.WorkRead) =>
    apply((state) => {
      const held = state.boards[listing.roomId];

      if (held === undefined || held.generation !== generation) return state;
      let next = state;

      for (const { thread } of listing.posts) {
        if (!held.removedPostIds.includes(thread.id) && !held.changedPostIds.includes(thread.id)) {
          next = work.receiveWorkThread(
            threads.revive(next, thread.id, read?.since ?? state.removalCount),
            thread,
            read,
            "read",
          );
        }
      }

      return boards.loadBoardListing(
        next,
        {
          ...listing,
          posts: listing.posts.map((summary) => ({
            ...summary,
            thread: next.threads[summary.thread.id] ?? summary.thread,
          })),
        },
        generation,
      );
    }),
  addBoardPost: (thread: Thread) => apply((state) => boards.addBoardPost(state, thread)),
  boardAutomationsChanged: (roomId: number) =>
    apply((state) => boards.boardAutomationsChanged(state, roomId)),
  startRead: (list: string, reload = true) => {
    const read = freshness.startRead(store.getState().freshness, list, reload);

    apply((state) => ({ ...state, freshness: read.freshness }));

    return read.ticket;
  },
  finishRead: (ticket: number) => {
    const read = freshness.finishRead(store.getState().freshness, ticket);

    apply((state) => ({ ...state, freshness: read.freshness }));
  },
  retireReads: (list: string) => {
    apply((state) => ({
      ...state,
      freshness: freshness.retireReads(state.freshness, list).freshness,
    }));
  },
  setBoot: (boot: Boot) => apply((state) => ({ ...state, boot })),
  setUploadLimit: (uploadLimitBytes: number) =>
    apply((state) =>
      state.boot === null
        ? state
        : {
            ...state,
            boot: { ...state.boot, account: { ...state.boot.account, uploadLimitBytes } },
          },
    ),
  setWorkspaceStyles: (css: string | null) => apply((state) => setWorkspaceStyles(state, css)),
  setWorkspaceBranding: (branding: WorkspaceBranding) =>
    apply((state) => setWorkspaceBranding(state, branding)),
  setMe: (me: Me) => apply((state) => reduce.setMe(state, me)),
  setConnection: (connection: ConnectionStatus) =>
    apply((state) => (state.connection === connection ? state : { ...state, connection })),
  mergeUsers: (users: readonly User[]) => apply((state) => reduce.mergeUsers(state, users)),
  setPresence: (list: readonly UserPresence[]) => apply((state) => reduce.setPresence(state, list)),
  setDndAllowance: (userId: number, allowed: boolean) =>
    apply((state) =>
      state.dndAllowances[userId] === allowed
        ? state
        : { ...state, dndAllowances: { ...state.dndAllowances, [userId]: allowed } },
    ),
  setSidebarLoading: () =>
    apply((state) => ({ ...state, sidebar: { ...state.sidebar, status: "loading" } })),
  setSidebarFailed: () =>
    apply((state) => ({ ...state, sidebar: { ...state.sidebar, status: "error" } })),
  /**
   * A request whose reply installs or removes sidebar rows begins: answers its ticket, held until
   * `closeRowTicket` (see `withRowTicket` in src/sync/row-ticket.ts).
   */
  openRowTicket: (): number => {
    const since = rowClock(store.getState());

    heldTickets.push(since);

    return since;
  },
  /** The request holding `since` settled; touches no request in flight needs are dropped. */
  closeRowTicket: (since: number) => {
    const index = heldTickets.indexOf(since);

    if (index >= 0) heldTickets.splice(index, 1);
    apply((state) => state);
  },
  /**
   * A whole-sidebar reply; `since` is its request's ticket. Answers whether it landed: a reply
   * that started before a sync resync is stale, and leaves the store as it is.
   */
  loadSidebar: (sidebar: Sidebar, since: number): boolean => {
    if (isStale(store.getState(), since)) return false;
    apply((state) => reduce.loadSidebar(state, sidebar, since));

    return true;
  },
  /**
   * A whole-sidebar snapshot the sync engine read (a resync), `since` its own ticket: it is
   * authoritative, so every HTTP reply already on its way becomes stale. A room whose membership
   * it installed or changed claims its outcome, as a `sidebar.row.upserted` does, so a room read
   * (a 404 above all) that started earlier can't land over it. Answers those rooms.
   */
  resyncSidebar: (sidebar: Sidebar, since: number): readonly number[] => {
    const before = store.getState().sidebar.rows;

    apply((state) => reduce.resyncSidebar(state, sidebar, since));

    const after = store.getState().sidebar.rows;
    const changed: number[] = [];

    for (const row of Object.values(after)) {
      if (membershipChanged(before[row.room.id], row)) {
        claimRoomOutcome(row.room.id, beginRoomRequest());
        dirtyRoomReread(row.room.id);
        changed.push(row.room.id);
      }
    }

    return changed;
  },
  /** Read here: newer than any HTTP reply already on its way, so that reply leaves the row be. */
  markRoomRead: (roomId: number) =>
    apply((state) => touchRows(reduce.markRoomRead(state, roomId), [roomId])),
  setRoomLoading: (roomId: number) => apply((state) => reduce.setRoomLoading(state, roomId)),
  /**
   * A room read failed. With `started` (the read's outcome sequence) it lands only when no newer
   * room outcome has, like a read's detail, preview or 404; answers whether it landed.
   */
  setRoomError: (roomId: number, error: string, started?: number): boolean => {
    if (started === undefined) {
      apply((state) => reduce.setRoomError(state, roomId, error));

      return true;
    }

    return landRoom(roomId, started, (state) => reduce.setRoomError(state, roomId, error));
  },
  setRoomPreview: (roomId: number, preview: OpenRoomPreview, started: number) =>
    landRoom(roomId, started, (state) => reduce.setRoomPreview(state, roomId, preview)),
  setRoomDetail: (detail: RoomDetail, started: number) =>
    landRoom(detail.room.id, started, (state) => reduce.setRoomDetail(state, detail)),
  /**
   * An HTTP 404 (or a delete or leave reply): access is gone, and the room shows as unavailable
   * whenever the room outcome is still this request's (`started`). The sidebar row is sync's
   * word: one a resync installed, or sync touched, after the request began (`rowsSince`, its row
   * ticket) stays, and the server publishes its removal if access really went.
   */
  setRoomUnavailable: (roomId: number, started: number, rowsSince: number) =>
    landRoom(roomId, started, (state) => {
      clearRoomJoin(roomId);

      return reduce.setRoomUnavailable(state, roomId, removalIsStale(state, roomId, rowsSince));
    }),
  applyPage: (roomId: number, page: MessagePage, mode: reduce.PageMode, request?: number) =>
    apply((state) => reduce.applyPage(state, roomId, page, mode, request)),
  setPageLoading: (roomId: number, direction: "older" | "newer") => {
    apply((state) => reduce.setPageLoading(state, roomId, direction));

    const timeline = store.getState().timelines[roomId];

    return direction === "older" ? (timeline?.olderRequest ?? 0) : (timeline?.newerRequest ?? 0);
  },
  setPageFailed: (roomId: number, direction?: "older" | "newer", request?: number) =>
    apply((state) => reduce.setPageFailed(state, roomId, direction, request)),
  setPageReplacing: (roomId: number) => apply((state) => reduce.setPageReplacing(state, roomId)),
  clearUnreadDivider: (roomId: number) =>
    apply((state) => reduce.clearUnreadDivider(state, roomId)),
  moveUnreadDivider: (roomId: number, fromId: number) =>
    apply((state) => reduce.moveUnreadDivider(state, roomId, fromId)),
  markUnreadFrom: (roomId: number, fromId: number, now: number, since: number) =>
    apply((state) => reduce.markUnreadFrom(state, roomId, fromId, now, since)),
  receiveMessage: (message: MessageDTO) => apply((state) => reduce.receiveMessage(state, message)),
  addPending: (pending: PendingMessage) => apply((state) => reduce.addPending(state, pending)),
  setPendingState: (
    clientMessageId: string,
    status: PendingMessage["state"],
    error: string | null,
  ) => apply((state) => reduce.setPendingState(state, clientMessageId, status, error)),
  discardPending: (clientMessageId: string) =>
    apply((state) => reduce.discardPending(state, clientMessageId)),
  applyEvents: (events: readonly SyncEvent[], now: number) => {
    noteRowEvents(events);
    apply((state) => (events.length === 0 ? state : reduce.applyEvents(state, events, now)));
  },
  /**
   * An HTTP reply's rows, as the local `sidebar.row.*` events that land them. `since` is the
   * request's ticket: a room the sync path changed after it keeps the store's row, and none land
   * once a resync made the reply stale.
   */
  landReplyRows: (events: readonly SyncEvent[], now: number, since: number) => {
    const fresh = untouchedReplyEvents(store.getState(), events, since);

    noteRowEvents(fresh);
    apply((state) => (fresh.length === 0 ? state : reduce.applyEvents(state, fresh, now, "reply")));
  },
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
  applyThreadPage: (threadId: number, page: MessagePage, mode: reduce.PageMode, request?: number) =>
    apply((state) => reduce.applyThreadPage(state, threadId, page, mode, request)),
  setThreadPageLoading: (threadId: number, direction: "older" | "newer") => {
    apply((state) => reduce.setThreadPageLoading(state, threadId, direction));

    const timeline = store.getState().threadTimelines[threadId];

    return direction === "older" ? (timeline?.olderRequest ?? 0) : (timeline?.newerRequest ?? 0);
  },
  setThreadPageFailed: (threadId: number, direction?: "older" | "newer", request?: number) =>
    apply((state) => reduce.setThreadPageFailed(state, threadId, direction, request)),
  clearThreadPageLoading: (
    threadId: number,
    direction: "older" | "newer" | "replace",
    request?: number,
  ) => apply((state) => reduce.clearThreadPageLoading(state, threadId, direction, request)),
  setThreadPageReplacing: (threadId: number) =>
    apply((state) => reduce.setThreadPageReplacing(state, threadId)),
  setThreadPaneLoading: (threadId: number) =>
    apply((state) => threads.setThreadPaneLoading(state, threadId)),
  setThreadPaneError: (threadId: number, error: string) =>
    apply((state) => threads.setThreadPaneError(state, threadId, error)),
  /** `since`: the state's `removalCount` when the request was sent. */
  loadThreadDetail: (
    detail: ThreadDetail,
    since: number = store.getState().removalCount,
    read?: work.WorkRead,
  ) => apply((state) => work.loadWorkThreadDetail(state, detail, read, since)),
  /** A thread started here: its pane data, and the first reply on its (new) timeline. */
  threadCreated: (
    created: ThreadCreated,
    since: number = store.getState().removalCount,
    read?: work.WorkRead,
  ) =>
    apply((state) =>
      reduce.receiveMessage(
        work.loadWorkThreadDetail(state, created.detail, read, since),
        created.message,
      ),
    ),
  mergeWorkFacts: (thread: Thread, read: work.WorkRead) =>
    apply((state) => {
      const held = state.threads[thread.id];

      return held === undefined || threads.removedSince(state, thread.id, read.since)
        ? state
        : work.receiveWorkThread(state, { ...held, work: thread.work }, read, "read");
    }),
  upsertThread: (thread: Thread) => apply((state) => work.receiveWorkThread(state, thread)),
  setThreadMembership: (threadId: number, membership: ThreadMembership | null) =>
    apply((state) => threads.setThreadMembership(state, threadId, membership)),
  setThreadListLoading: (roomId: number, filter: ThreadFilter) =>
    apply((state) => threads.setThreadListLoading(state, roomId, filter)),
  setThreadListFailed: (roomId: number, filter: ThreadFilter) =>
    apply((state) => threads.setThreadListFailed(state, roomId, filter)),
  loadThreadList: (
    roomId: number,
    filter: ThreadFilter,
    list: ThreadList,
    since: number = store.getState().removalCount,
    read?: work.WorkRead,
  ) =>
    apply((state) => {
      let next = state;

      const summaries = list.threads
        .filter(({ thread }) => !threads.removedSince(state, thread.id, since))
        .map((summary) => {
          next = work.receiveWorkThread(
            threads.revive(next, summary.thread.id, since),
            summary.thread,
            read,
            "read",
          );

          return { ...summary, thread: next.threads[summary.thread.id] ?? summary.thread };
        });

      return threads.loadThreadList(next, roomId, filter, { ...list, threads: summaries }, since);
    }),
  setHuddlePresence: (presence: HuddlePresence) =>
    apply((state) => huddles.setHuddlePresence(state, presence)),
  loadHuddlePresence: (list: HuddlePresenceList) =>
    apply((state) => huddles.loadHuddlePresence(state, list)),
  setStage: (stage: StageState) => apply((state) => huddles.setStage(state, stage)),
  loadStageDetail: (detail: StageDetail) =>
    apply((state) => huddles.loadStageDetail(state, detail)),
  // --- S3: the activity inbox, saved items and scheduled messages ---
  beginActivityGeneration: (newEpoch = true) =>
    apply((state) => activity.beginActivityGeneration(state, newEpoch)),
  setActivityListLoading: (tab: ActivityTab, status: ActivityState, more: boolean) =>
    apply((state) => activity.setActivityListLoading(state, tab, status, more)),
  setActivityListFailed: (
    tab: ActivityTab,
    status: ActivityState,
    error: string,
    generation?: number,
    activityGeneration?: number,
  ) =>
    apply((state) =>
      activity.setActivityListFailed(state, tab, status, error, generation, activityGeneration),
    ),
  landActivityPage: (
    tab: ActivityTab,
    status: ActivityState,
    page: ActivityList,
    mode: "replace" | "more",
    start?: activity.ActivityLoadStart,
  ) => apply((state) => activity.landActivityPage(state, tab, status, page, mode, start)),
  /** An item as the server has it now; `unread` `null` keeps the badge. */
  applyActivityItem: (item: ActivityItem, unread: ActivityUnreadCount | null) =>
    apply((state) => activity.applyActivityItem(state, item, unread)),
  /** A change on its way, shown at once (see `activity.showActivityChange`). */
  showActivityChange: (item: ActivityItem, token: number, unreadDelta: number) =>
    apply((state) => activity.showActivityChange(state, item, token, unreadDelta)),
  /** A change on its way ended (see `activity.endActivityChange`). */
  endActivityChange: (end: activity.ActivityChangeEnd) =>
    apply((state) => activity.endActivityChange(state, end)),
  setActivityUnreadCount: (unread: ActivityUnreadCount, generation?: number) =>
    apply((state) => activity.setActivityUnreadCount(state, unread, generation)),
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
  /** Every activity list reloads when next shown (a room came into the sidebar). */
  markActivityStale: () => apply((state) => activity.markActivityStale(state)),
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
  rollbackWork: (threadId: number, shown: WorkFacts | null) =>
    apply((state) => work.rollbackWork(state, threadId, shown)),
  landWorkReply: (
    detail: ThreadDetail,
    shown: WorkFacts | null | undefined,
    read?: work.WorkRead,
  ) => apply((state) => work.landWorkReply(state, detail, shown, read)),
  countWorkWrite: (threadId: number, delta: 1 | -1) =>
    apply((state) => work.countWorkWrite(state, threadId, delta)),
  setWorkListLoading: (filter: WorkFilter) =>
    apply((state) => work.setWorkListLoading(state, filter)),
  setWorkListFailed: (filter: WorkFilter, error: string, generation: number) =>
    apply((state) => work.setWorkListFailed(state, filter, error, generation)),
  landWorkList: (filter: WorkFilter, list: WorkList, generation: number, read?: work.WorkRead) =>
    apply((state) => work.landWorkList(state, filter, list, generation, read)),
  // --- S4: agents ---
  setAgentDirectoryLoading: () => apply((state) => agents.setDirectoryLoading(state)),
  landAgentDirectory: (page: AgentDirectory, generation: number, read?: agents.AgentRead) =>
    apply((state) => agents.landDirectory(state, page, generation, read)),
  setAgentDirectoryFailed: (error: string, generation: number) =>
    apply((state) => agents.setDirectoryFailed(state, error, generation)),
  setAgentProfileLoading: (agentId: number) =>
    apply((state) => agents.setProfileLoading(state, agentId)),
  landAgentProfile: (profile: AgentProfile, generation: number, read?: agents.AgentRead) =>
    apply((state) => agents.landProfile(state, profile, generation, read)),
  setAgentProfileFailed: (agentId: number, error: string, missing: boolean, generation: number) =>
    apply((state) => agents.setProfileFailed(state, agentId, error, missing, generation)),
  setApprovalListLoading: (key: approvals.ApprovalListKey, more: boolean) =>
    apply((state) => approvals.setApprovalListLoading(state, key, more)),
  setApprovalListFailed: (key: approvals.ApprovalListKey, error: string, generation?: number) =>
    apply((state) => approvals.setApprovalListFailed(state, key, error, generation)),
  landApprovalPage: (
    key: approvals.ApprovalListKey,
    page: AgentApprovalPage,
    mode: "replace" | "more",
    generation?: number,
    ticket?: number,
    read?: approvals.ApprovalRead,
  ) =>
    apply((state) => approvals.landApprovalPage(state, key, page, mode, generation, ticket, read)),
  applyApproval: (approval: AgentApproval, read?: approvals.ApprovalRead) =>
    apply((state) => approvals.applyApproval(state, approval, undefined, read)),
  settleApproval: (approval: AgentApproval, read?: approvals.ApprovalRead) =>
    apply((state) => approvals.settleApproval(state, approval, read)),
  showApproval: (approval: AgentApproval) =>
    apply((state) => approvals.showApproval(state, approval)),
  rollbackApproval: (shown: AgentApproval, before: AgentApproval) =>
    apply((state) => approvals.rollbackApproval(state, shown, before)),
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
  /** Sidebar organisation (S3): pending changes, category replies, membership replies. */
  addSidebarOverlay: (entry: organize.SidebarOverlay) =>
    apply((state) => organize.addOverlay(state, entry)),
  dropSidebarOverlay: (entry: organize.SidebarOverlay) =>
    apply((state) => organize.dropOverlay(state, entry)),
  /** A rename or fold reply; `since` is its request's ticket. */
  landCategory: (category: RoomCategory, since: number) =>
    apply((state) => organize.landCategory(state, category, since)),
  /** An organising reply's rows the sidebar has; `since` is the request's ticket. */
  mergeOrganization: (rows: readonly SidebarRow[], since: number) =>
    apply((state) => organize.mergeOrganization(state, rows, since)),
  /** A create reply; `since` is its request's ticket. */
  landCreatedCategory: (
    category: RoomCategory,
    draft: organize.SidebarOverlay,
    settled: organize.SidebarOverlay,
    since: number,
  ) => apply((state) => organize.landCreatedCategory(state, category, draft, settled, since)),
  /** A reorder reply's list; `since` is its request's ticket. */
  landCategories: (categories: readonly RoomCategory[], since: number) =>
    apply((state) => organize.landCategories(state, categories, since)),
  /** A delete reply; `since` is its request's ticket. */
  landCategoryRemoval: (categoryId: number, since: number) =>
    apply((state) => organize.landCategoryRemoval(state, categoryId, since)),
  /** A membership reply; `since` is its request's ticket. */
  setMembership: (membership: Membership, since: number) =>
    apply((state) => organize.setMembership(state, membership, since)),
  /** Back to an empty store (tests). */
  reset: () => {
    heldTickets.length = 0;
    apply(() => initialState);
  },
};

export type Mutations = typeof mutations;
