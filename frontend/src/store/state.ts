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

/**
 * The whole live store. Normalized: every entity lives once, by id; views hold ids. The sync
 * engine writes it from outside React in batches (one `setState` per batch, so one commit), and
 * components read slices through selectors.
 */
export interface State {
  readonly boot: Boot | null;
  readonly me: Me | null;
  readonly connection: ConnectionStatus;
  readonly users: Readonly<Record<number, User>>;
  readonly presence: Readonly<Record<number, UserPresence>>;
  readonly sidebar: SidebarState;
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
  readonly roomThreads: Readonly<Record<number, RoomThreadList>>;
  /** Typists per topic (`room:12`): user id to the time (ms) their entry expires. */
  readonly typing: Readonly<Record<string, Readonly<Record<number, number>>>>;
  /** Deleted message ids and when (ms) their tombstone lapses: a late update can't revive them. */
  readonly tombstones: Readonly<Record<number, number>>;
}

export interface SidebarState {
  readonly status: LoadStatus;
  /** Room ids in the server's order (`LOWER(rooms.name)`). */
  readonly order: readonly number[];
  readonly rows: Readonly<Record<number, SidebarRow>>;
  readonly categories: readonly RoomCategory[];
  readonly placeholderUserIds: readonly number[];
  readonly canCreateRooms: boolean;
}

export const initialState: State = {
  boot: null,
  me: null,
  connection: "connecting",
  users: {},
  presence: {},
  sidebar: {
    status: "idle",
    order: [],
    rows: {},
    categories: [],
    placeholderUserIds: [],
    canCreateRooms: false,
  },
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
  roomThreads: {},
  typing: {},
  tombstones: {},
};

export const emptyTimeline: Timeline = {
  ids: [],
  before: null,
  after: null,
  status: "idle",
  loadingOlder: false,
  loadingNewer: false,
  unreadFromId: null,
  unreadCount: 0,
  generation: 0,
};

/** How long a remote typist stays listed without a refresh. */
export const TYPING_TTL_MS = 6000;

/** How long a deleted message's tombstone holds. */
export const TOMBSTONE_TTL_MS = 60_000;
