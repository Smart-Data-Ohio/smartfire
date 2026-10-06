import {
  createRootRoute,
  createRoute,
  createRouter,
  lazyRouteComponent,
  notFound,
  Outlet,
} from "@tanstack/react-router";
import { RoomRoute } from "./features/room/room-route.tsx";
import { AppShell } from "./features/shell/app-shell.tsx";
import { HomeView } from "./features/shell/home-view.tsx";

/** A path segment that must be a positive integer id; anything else is a 404. */
function parseId(segment: string): number {
  const id = Number(segment);

  if (!Number.isSafeInteger(id) || id <= 0) {
    throw notFound();
  }

  return id;
}

const rootRoute = createRootRoute({ component: Outlet });

/** The design-system gallery, its own chunk so none of it ships in the entry. */
const kitchenSinkRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "_kitchen-sink",
  component: lazyRouteComponent(() => import("./routes/kitchen-sink/kitchen-sink.tsx")),
});

const shellRoute = createRoute({
  getParentRoute: () => rootRoute,
  id: "shell",
  component: AppShell,
});

/** `/app/`: the last room on wide screens, the conversation list on phones. */
const homeRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/",
  component: HomeView,
});

/** `/app/r/$roomId`: a room's timeline (channel, DM, voice and stage share it). */
const roomRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "r/$roomId",
  params: {
    parse: ({ roomId }) => ({ roomId: parseId(roomId) }),
    stringify: ({ roomId }) => ({ roomId: `${roomId}` }),
  },
  component: RoomRoute,
});

/** `/app/r/$roomId/m/$messageId`: the same room, scrolled to and flashing one message. */
const permalinkRoute = createRoute({
  getParentRoute: () => roomRoute,
  path: "m/$messageId",
  params: {
    parse: ({ messageId }) => ({ messageId: parseId(messageId) }),
    stringify: ({ messageId }) => ({ messageId: `${messageId}` }),
  },
  component: () => null,
});

/** The new-thread pane's query as the URL has it. */
interface RawNewThreadSearch {
  readonly parent?: unknown;
}

/** The new-thread pane's query: the root message the thread starts on. */
export interface NewThreadSearch {
  readonly parent: number;
}

/** `/app/r/$roomId/t/new?parent=`: the right pane drafting a thread's first reply on `parent`. */
const newThreadRoute = createRoute({
  getParentRoute: () => roomRoute,
  path: "t/new",
  validateSearch: (search: RawNewThreadSearch): NewThreadSearch => ({
    parent: parseId(String(search.parent ?? "")),
  }),
  component: () => null,
});

/** The thread pane's query as the URL has it. */
interface RawThreadSearch {
  readonly m?: unknown;
}

/** The thread pane's query: the reply a permalink points at, if any. */
export interface ThreadSearch {
  readonly m?: number;
}

/**
 * `/app/r/$roomId/t/$threadId`: the room with a thread open in the right pane; `?m=` scrolls to
 * and highlights one reply (a reply's permalink).
 */
const threadRoute = createRoute({
  getParentRoute: () => roomRoute,
  path: "t/$threadId",
  validateSearch: (search: RawThreadSearch): ThreadSearch => {
    const m = Number(search.m);

    // Anything but a positive integer id is ignored: the thread opens at its newest reply.
    return Number.isSafeInteger(m) && m > 0 ? { m } : {};
  },
  params: {
    parse: ({ threadId }) => ({ threadId: parseId(threadId) }),
    stringify: ({ threadId }) => ({ threadId: `${threadId}` }),
  },
  component: () => null,
});

const routeTree = rootRoute.addChildren([
  kitchenSinkRoute,
  shellRoute.addChildren([
    homeRoute,
    roomRoute.addChildren([permalinkRoute, newThreadRoute, threadRoute]),
  ]),
]);

export const router = createRouter({
  routeTree,
  basepath: import.meta.env.BASE_URL,
  defaultPreload: false,
  scrollRestoration: false,
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
