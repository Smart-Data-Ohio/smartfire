import type { ConversationName } from "../gen/ConversationName.ts";
import type { HuddlePresence } from "../gen/HuddlePresence.ts";
import type { NotificationSettings } from "../gen/NotificationSettings.ts";
import type { StageState } from "../gen/StageState.ts";
import { type ActivitySlice, emptyActivity } from "./activity.ts";
import { type AgentsSlice, emptyAgents } from "./agents.ts";
import { type ApprovalsSlice, emptyApprovals } from "./approvals.ts";
import type { BoardState } from "./boards.ts";
import { type CardsState, emptyCards } from "./cards.ts";
import { emptyFreshness, type Freshness } from "./freshness.ts";
import { emptyLedger, type LedgerSlice } from "./ledger.ts";
import type {
  Boot,
  ConnectionStatus,
  LoadStatus,
  Me,
  MessageDTO,
  PendingMessage,
  RoomCategory,
  RoomState,
  RoomThreadList,
  SidebarRow,
  Thread,
  ThreadMembership,
  ThreadPaneState,
  Timeline,
  User,
  UserPresence,
} from "./model.ts";
import { emptyOverlay, type SidebarOverlay } from "./organize.ts";
import { noRowTouches, type RowTouches } from "./row-touches.ts";
import { emptySavedList, type SavedListSlice } from "./saved-list.ts";
import { emptyScheduled, type ScheduledSlice } from "./scheduled.ts";
import { emptyWork, type WorkSlice } from "./work.ts";

/**
 * The whole live store. Normalized: every entity lives once, by id; views hold ids. The sync
 * engine writes it from outside React in batches (one `setState` per batch, so one commit), and
 * components read slices through selectors.
 */
export interface State {
  readonly freshness: Freshness;
  readonly boot: Boot | null;
  readonly me: Me | null;
  readonly connection: ConnectionStatus;
  readonly users: Readonly<Record<number, User>>;
  readonly presence: Readonly<Record<number, UserPresence>>;
  /**
   * Whether the viewer lets each person through Do Not Disturb (`dnd_allowed_users`), by user
   * id, for the people the person pages have seen. Their own table, so not on `users`.
   */
  readonly dndAllowances: Readonly<Record<number, boolean>>;
  readonly sidebar: SidebarState;
  /** When the sync path last changed each sidebar row, so an older HTTP reply leaves it be. */
  readonly rowTouches: RowTouches;
  readonly rooms: Readonly<Record<number, RoomState>>;
  readonly messages: Readonly<Record<number, MessageDTO>>;
  readonly timelines: Readonly<Record<number, Timeline>>;
  /** By client message id. */
  readonly pending: Readonly<Record<string, PendingMessage>>;
  /** Pending client message ids per room (root timeline only), oldest first. */
  readonly pendingByRoom: Readonly<Record<number, readonly string[]>>;
  /** Pending client message ids per thread, oldest first. */
  readonly pendingByThread: Readonly<Record<number, readonly string[]>>;
  /** The viewer's saved items: message id to saved item id. */
  readonly saved: Readonly<Record<number, number>>;
  readonly threads: Readonly<Record<number, Thread>>;
  /** The viewer's membership per thread: absent until known, `null` when not a member. */
  readonly threadMemberships: Readonly<Record<number, ThreadMembership | null>>;
  readonly threadPanes: Readonly<Record<number, ThreadPaneState>>;
  /** Each open thread's loaded window of replies (the unread fields stay unused). */
  readonly threadTimelines: Readonly<Record<number, Timeline>>;
  readonly boards: Readonly<Record<number, BoardState>>;
  readonly roomThreads: Readonly<Record<number, RoomThreadList>>;
  /** Typists per topic (`room:12`): user id to the time (ms) their entry expires. */
  readonly typing: Readonly<Record<string, Readonly<Record<number, number>>>>;
  /** Deleted message ids and when (ms) their tombstone lapses: a late update can't revive them. */
  readonly tombstones: Readonly<Record<number, number>>;
  /**
   * Deleted thread ids, each with the `removalCount` its removal made: a `thread.created` or
   * `thread.updated` published out of order after the `thread.removed` can't bring the thread
   * back, and neither can an HTTP reply to a request sent before the removal. Only a reply to one
   * sent after it does (see `revive`). At most `MAX_REMOVED_THREADS`, the oldest dropped first.
   */
  readonly removedThreads: Readonly<Record<number, number>>;
  /** How many thread removals this session has seen: a request's `since` is the count at send. */
  readonly removalCount: number;
  /**
   * The newest removal dropped from `removedThreads`: a reply to a request sent before it can't
   * tell whether a thread it shows was removed meanwhile, so it doesn't add threads (see
   * `removedSince`).
   */
  readonly forgottenRemoval: number;
  /**
   * How many `board.automations.changed` events each board has had: an open automations pane
   * refetches when its board's count moves.
   */
  readonly boardAutomationsChanged: Readonly<Record<number, number>>;
  /** Who is in each room's call, by room id; rooms with nobody in their call are absent. */
  readonly huddles: Readonly<Record<number, HuddlePresence>>;
  /** Each loaded stage's roster and live stream, by room id. */
  readonly stages: Readonly<Record<number, StageState>>;
  /** The activity inbox: items, per-tab-and-state lists, the unread badge (S3). */
  readonly activity: ActivitySlice;
  /** The Saved page: saved items and per-filter lists (S3). */
  readonly savedList: SavedListSlice;
  /** Scheduled messages and their lists: pending, past, per room (S3). */
  readonly scheduled: ScheduledSlice;
  /** Names for cross-room rows, by `conversationKey(roomId, threadId)` (S3). */
  readonly conversationNames: Readonly<Record<string, ConversationName>>;
  /** Work detail for open thread panes and the work lists (S4); facts live on threads. */
  readonly work: WorkSlice;
  /** Agents: the directory, profiles and working presence (S4). */
  readonly agents: AgentsSlice;
  /** Agents' approval requests and their per-agent lists (S4). */
  readonly approvals: ApprovalsSlice;
  /** Agents' event ledgers: entries and per-agent lists (S4). */
  readonly ledger: LedgerSlice;
  /** Ballots, votes on their way and per-viewer card previews (see `cards.ts`). */
  readonly cards: CardsState;
}

export interface SidebarState {
  readonly notificationPreferences?: NotificationSettings;
  readonly notificationClock?: number;
  readonly status: LoadStatus;
  /** Room ids in the server's order (`LOWER(rooms.name)`). */
  readonly order: readonly number[];
  readonly rows: Readonly<Record<number, SidebarRow>>;
  readonly categories: readonly RoomCategory[];
  readonly placeholderUserIds: readonly number[];
  readonly canCreateRooms: boolean;
  /** Organising changes on their way to the server, drawn over the rows (S3, organize.ts). */
  readonly overlay: SidebarOverlay;
}

export const initialState: State = {
  freshness: emptyFreshness,
  boot: null,
  me: null,
  connection: "connecting",
  users: {},
  presence: {},
  dndAllowances: {},
  sidebar: {
    status: "idle",
    order: [],
    rows: {},
    categories: [],
    placeholderUserIds: [],
    canCreateRooms: false,
    overlay: emptyOverlay,
  },
  rowTouches: noRowTouches,
  rooms: {},
  messages: {},
  timelines: {},
  pending: {},
  pendingByRoom: {},
  pendingByThread: {},
  saved: {},
  threads: {},
  threadMemberships: {},
  threadPanes: {},
  threadTimelines: {},
  boards: {},
  roomThreads: {},
  typing: {},
  tombstones: {},
  removedThreads: {},
  removalCount: 0,
  forgottenRemoval: 0,
  boardAutomationsChanged: {},
  huddles: {},
  stages: {},
  activity: emptyActivity,
  savedList: emptySavedList,
  scheduled: emptyScheduled,
  conversationNames: {},
  work: emptyWork,
  agents: emptyAgents,
  approvals: emptyApprovals,
  ledger: emptyLedger,
  cards: emptyCards,
};

export const emptyTimeline: Timeline = {
  ids: [],
  before: null,
  after: null,
  status: "idle",
  loadingOlder: false,
  loadingNewer: false,
  olderRequest: 0,
  newerRequest: 0,
  unreadFromId: null,
  unreadCount: 0,
  generation: 0,
  arrived: null,
};

/** How long a remote typist stays listed without a refresh. */
export const TYPING_TTL_MS = 6000;

/** How long a deleted message's tombstone holds. */
export const TOMBSTONE_TTL_MS = 60_000;
