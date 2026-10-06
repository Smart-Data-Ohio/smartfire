import type {
  Boot,
  ConnectionStatus,
  LoadStatus,
  Me,
  MessageDTO,
  PendingMessage,
  RoomCategory,
  RoomState,
  SidebarRow,
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
  /** Pending client message ids per room, oldest first. */
  readonly pendingByRoom: Readonly<Record<number, readonly string[]>>;
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
