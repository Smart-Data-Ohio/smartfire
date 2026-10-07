import {
  createRootRoute,
  createRoute,
  createRouter,
  lazyRouteComponent,
  notFound,
  Outlet,
} from "@tanstack/react-router";
import { parseActivitySearch } from "./features/activity/activity-search.ts";
import { RoomRoute } from "./features/room/room-route.tsx";
import { parseSavedSearch } from "./features/saved/saved-search.ts";
import { AppShell } from "./features/shell/app-shell.tsx";
import { HomeView } from "./features/shell/home-view.tsx";
import { NotFound } from "./features/shell/not-found.tsx";

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

/** `/app/activity?tab=&status=`: the activity inbox (its own chunk). */
const activityRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "activity",
  validateSearch: parseActivitySearch,
  component: lazyRouteComponent(
    () => import("./features/activity/activity-route.tsx"),
    "ActivityRoute",
  ),
});

/** `/app/saved?status=`: saved messages (Slack's "Later"). */
const savedRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "saved",
  validateSearch: parseSavedSearch,
  component: lazyRouteComponent(() => import("./features/saved/saved-route.tsx"), "SavedRoute"),
});

/** `/app/scheduled`: every scheduled message, upcoming, stranded and past. */
const scheduledRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "scheduled",
  component: lazyRouteComponent(
    () => import("./features/scheduled/scheduled-page.tsx"),
    "ScheduledPage",
  ),
});

/** `/app/agents`: every agent in the workspace, with live status (S4). */
const agentsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "agents",
  component: lazyRouteComponent(
    () => import("./features/agents/agent-directory-page.tsx"),
    "AgentDirectoryPage",
  ),
});

/** `/app/agents/$agentId`: an agent's profile (S4). */
const agentRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "agents/$agentId",
  params: {
    parse: ({ agentId }) => ({ agentId: parseId(agentId) }),
    stringify: ({ agentId }) => ({ agentId: `${agentId}` }),
  },
  component: lazyRouteComponent(
    () => import("./features/agents/agent-profile-route.tsx"),
    "AgentProfileRoute",
  ),
});

const routeTree = rootRoute.addChildren([
  kitchenSinkRoute,
  shellRoute.addChildren([
    homeRoute,
    activityRoute,
    savedRoute,
    scheduledRoute,
    agentsRoute,
    agentRoute,
    roomRoute.addChildren([permalinkRoute, newThreadRoute, threadRoute]),
  ]),
]);

export const router = createRouter({
  routeTree,
  basepath: import.meta.env.BASE_URL,
  defaultPreload: false,
  // A destination the SPA hasn't ported yet opens on its classic page (src/lib/screens.ts).
  defaultNotFoundComponent: NotFound,
  scrollRestoration: false,
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
