import {
  createRootRoute,
  createRoute,
  createRouter,
  lazyRouteComponent,
  notFound,
  Outlet,
} from "@tanstack/react-router";
import { parseActivitySearch } from "./features/activity/activity-search.ts";
import { AdminView } from "./features/admin/admin-view.tsx";
import { AuditLogSection } from "./features/admin/audit-log-section.tsx";
import { BotCredentialsSection } from "./features/admin/bot-credentials-section.tsx";
import { BotGrantsSection } from "./features/admin/bot-grants-section.tsx";
import { BotNewSection } from "./features/admin/bot-new-section.tsx";
import { BotSection } from "./features/admin/bot-section.tsx";
import { BotsSection } from "./features/admin/bots-section.tsx";
import { IconsSection } from "./features/admin/icons-section.tsx";
import { IntegrationsSection as AdminIntegrationsSection } from "./features/admin/integrations-section.tsx";
import { PeopleSection } from "./features/admin/people-section.tsx";
import { StylesSection } from "./features/admin/styles-section.tsx";
import { WorkspaceSection } from "./features/admin/workspace-section.tsx";
import { MessageResolver } from "./features/room/message-resolver.tsx";
import { RoomRoute } from "./features/room/room-route.tsx";
import { parseSavedSearch } from "./features/saved/saved-search.ts";
import { AppearanceSection } from "./features/settings/appearance-section.tsx";
import { CallsSection } from "./features/settings/calls-section.tsx";
import { DevicesSection } from "./features/settings/devices-section.tsx";
import { IntegrationsSection } from "./features/settings/integrations-section.tsx";
import { NotificationsSection } from "./features/settings/notifications-section.tsx";
import { ProfileSection } from "./features/settings/profile-section.tsx";
import { RoomsSection } from "./features/settings/rooms-section.tsx";
import { SecuritySection } from "./features/settings/security-section.tsx";
import { SessionsSection } from "./features/settings/sessions-section.tsx";
import { SettingsView } from "./features/settings/settings-view.tsx";
import { StatusSection } from "./features/settings/status-section.tsx";
import { AppShell } from "./features/shell/app-shell.tsx";
import { HomeView } from "./features/shell/home-view.tsx";
import { NotFound } from "./features/shell/not-found.tsx";
import {
  PersonalSlackRunSection,
  PersonalSlackSection,
} from "./features/slack/personal-slack-section.tsx";
import { parseRunSearch } from "./features/slack/slack-format.ts";
import { SlackPlanSection } from "./features/slack/slack-plan-section.tsx";
import { SlackRunSection, SlackRunsSection } from "./features/slack/slack-runs-section.tsx";
import { SlackSetupSection } from "./features/slack/slack-setup-section.tsx";

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

/** Bare classic message links resolve their conversation before opening its permalink. */
const messageRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "m/$messageId",
  params: {
    parse: ({ messageId }) => ({ messageId: parseId(messageId) }),
    stringify: ({ messageId }) => ({ messageId: `${messageId}` }),
  },
  component: MessageResolver,
});

/** Existing room controls, opened by their classic page's URL. */
const roomControlRoutes = [
  createRoute({ getParentRoute: () => roomRoute, path: "threads", component: () => null }),
  createRoute({ getParentRoute: () => roomRoute, path: "files", component: () => null }),
  createRoute({ getParentRoute: () => roomRoute, path: "pins", component: () => null }),
  createRoute({ getParentRoute: () => roomRoute, path: "notifications", component: () => null }),
];

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

/** `/app/settings`: the classic profile page's sections, the profile first. */
const settingsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "settings",
  component: SettingsView,
});

/** The settings sections, each at `/app/settings/<path>` (the profile at `/app/settings`). */
const settingsSections = [
  createRoute({ getParentRoute: () => settingsRoute, path: "/", component: ProfileSection }),
  createRoute({ getParentRoute: () => settingsRoute, path: "status", component: StatusSection }),
  createRoute({
    getParentRoute: () => settingsRoute,
    path: "notifications",
    component: NotificationsSection,
  }),
  createRoute({ getParentRoute: () => settingsRoute, path: "rooms", component: RoomsSection }),
  createRoute({
    getParentRoute: () => settingsRoute,
    path: "appearance",
    component: AppearanceSection,
  }),
  createRoute({ getParentRoute: () => settingsRoute, path: "calls", component: CallsSection }),
  createRoute({
    getParentRoute: () => settingsRoute,
    path: "security",
    component: SecuritySection,
  }),
  createRoute({
    getParentRoute: () => settingsRoute,
    path: "sessions",
    component: SessionsSection,
  }),
  createRoute({ getParentRoute: () => settingsRoute, path: "devices", component: DevicesSection }),
  createRoute({
    getParentRoute: () => settingsRoute,
    path: "integrations",
    component: IntegrationsSection,
  }),
  createRoute({
    getParentRoute: () => settingsRoute,
    path: "slack",
    component: PersonalSlackSection,
  }),
  createRoute({
    getParentRoute: () => settingsRoute,
    path: "slack/$runId",
    component: PersonalSlackRunSection,
  }),
] as const;

/** `/app/admin`: the classic account pages, the workspace first. */
const adminRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "admin",
  component: AdminView,
});

/** The admin sections, each at `/app/admin/<path>` (the workspace at `/app/admin`). */
const adminSections = [
  createRoute({ getParentRoute: () => adminRoute, path: "/", component: WorkspaceSection }),
  createRoute({ getParentRoute: () => adminRoute, path: "people", component: PeopleSection }),
  createRoute({ getParentRoute: () => adminRoute, path: "icons", component: IconsSection }),
  createRoute({ getParentRoute: () => adminRoute, path: "styles", component: StylesSection }),
  createRoute({ getParentRoute: () => adminRoute, path: "audit-log", component: AuditLogSection }),
  createRoute({
    getParentRoute: () => adminRoute,
    path: "integrations",
    component: AdminIntegrationsSection,
  }),
  createRoute({ getParentRoute: () => adminRoute, path: "bots", component: BotsSection }),
  createRoute({ getParentRoute: () => adminRoute, path: "bots/new", component: BotNewSection }),
  createRoute({ getParentRoute: () => adminRoute, path: "bots/$botId", component: BotSection }),
  createRoute({
    getParentRoute: () => adminRoute,
    path: "bots/$botId/credentials",
    component: BotCredentialsSection,
  }),
  createRoute({
    getParentRoute: () => adminRoute,
    path: "bots/$botId/grants",
    component: BotGrantsSection,
  }),
  createRoute({ getParentRoute: () => adminRoute, path: "slack", component: SlackSetupSection }),
  createRoute({
    getParentRoute: () => adminRoute,
    path: "slack/runs",
    component: SlackRunsSection,
  }),
  createRoute({
    getParentRoute: () => adminRoute,
    path: "slack/runs/$runId",
    validateSearch: parseRunSearch,
    component: SlackRunSection,
  }),
  createRoute({
    getParentRoute: () => adminRoute,
    path: "slack/runs/$runId/plan",
    component: SlackPlanSection,
  }),
] as const;

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

/** `/app/people`: the workspace's people, to message or huddle with (its own chunk). */
const peopleRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "people",
  component: lazyRouteComponent(() => import("./features/people/people-page.tsx"), "PeoplePage"),
});

/** `/app/people/$userId`: someone's page (a bot's opens its agent or classic page). */
const personRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "people/$userId",
  params: {
    parse: ({ userId }) => ({ userId: parseId(userId) }),
    stringify: ({ userId }) => ({ userId: `${userId}` }),
  },
  component: lazyRouteComponent(() => import("./features/people/person-page.tsx"), "PersonRoute"),
});

/** The search page's query as the URL has it. */
interface RawSearchPageSearch {
  readonly q?: unknown;
}

/** The search page's query: what was searched for ("" or absent for none yet). */
export interface SearchPageSearch {
  readonly q?: string;
}

/** `/app/search?q=`: global search, its own chunk. */
const searchRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "search",
  validateSearch: (search: RawSearchPageSearch): SearchPageSearch =>
    search.q === undefined || search.q === null || search.q === "" ? {} : { q: String(search.q) },
  component: lazyRouteComponent(() => import("./features/search/search-page.tsx"), "SearchPage"),
});

const routeTree = rootRoute.addChildren([
  kitchenSinkRoute,
  shellRoute.addChildren([
    homeRoute,
    activityRoute,
    savedRoute,
    scheduledRoute,
    searchRoute,
    peopleRoute,
    personRoute,
    messageRoute,
    roomRoute.addChildren([permalinkRoute, newThreadRoute, threadRoute, ...roomControlRoutes]),
    settingsRoute.addChildren(settingsSections),
    adminRoute.addChildren(adminSections),
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
