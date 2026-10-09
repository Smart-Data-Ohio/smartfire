import {
  createRootRoute,
  createRoute,
  createRouter,
  notFound,
  Outlet,
} from "@tanstack/react-router";
import type { ComponentType } from "react";
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
import { parseApprovalsSearch, parseLedgerSearch } from "./features/agents/agent-search.ts";
import { captureInitialMessageLink } from "./features/room/message-link.ts";
import { RoomRoute } from "./features/room/room-route.tsx";
import { NewRoomRoute } from "./features/rooms/new-room-route.tsx";
import { NEW_ROOM_SLUGS } from "./features/rooms/room-forms.ts";
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
  ROUTE_PENDING_DELAY_MS,
  ROUTE_PENDING_MIN_MS,
  RoutePending,
} from "./features/shell/route-pending.tsx";
import {
  PersonalSlackRunSection,
  PersonalSlackSection,
} from "./features/slack/personal-slack-section.tsx";
import { parseRunSearch } from "./features/slack/slack-format.ts";
import { SlackPlanSection } from "./features/slack/slack-plan-section.tsx";
import { SlackRunSection, SlackRunsSection } from "./features/slack/slack-runs-section.tsx";
import { SlackSetupSection } from "./features/slack/slack-setup-section.tsx";
import { parseWorkSearch } from "./features/work/work-search.ts";
import { parseRoomSearch } from "./lib/board-search.ts";
import { isModuleResourceLoadError, loadForUpdate } from "./service-worker/update-required.ts";

export type { BoardSearch } from "./lib/board-search.ts";

/** A path segment that must be a positive integer id; anything else is a 404. */
function parseId(segment: string): number {
  const id = Number(segment);

  if (!Number.isSafeInteger(id) || id <= 0) {
    throw notFound();
  }

  return id;
}

/** A screen in its own chunk: the loader fetches it, and the component renders without suspending. */
function chunked(load: () => Promise<{ default: ComponentType }>) {
  let View: ComponentType | null = null;

  const loader = async () => {
    try {
      const module = await loadForUpdate(load);

      View = module.default;
    } catch (error) {
      if (!isModuleResourceLoadError(error)) {
        throw error;
      }

      View = () => null;
    }
  };

  const component = () => (View === null ? null : <View />);

  return { loader, component };
}

const rootRoute = createRootRoute({ component: Outlet });

/** The design-system gallery, its own chunk so none of it ships in the entry. */
const kitchenSinkRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "_kitchen-sink",
  ...chunked(() => import("./routes/kitchen-sink/kitchen-sink.tsx")),
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
  validateSearch: parseRoomSearch,
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

/** The classic handoff page names only the thread: resolve its room, then open the dialog. */
const handoffResolverRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "t/$threadId/handoff",
  params: {
    parse: ({ threadId }) => ({ threadId: parseId(threadId) }),
    stringify: ({ threadId }) => ({ threadId: `${threadId}` }),
  },
  ...chunked(() =>
    import("./features/work/handoff-resolver.tsx").then((module) => ({
      default: module.HandoffResolver,
    })),
  ),
});

/** The classic links page names only the thread: resolve its room, then open the form. */
const linksResolverRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "t/$threadId/links",
  params: {
    parse: ({ threadId }) => ({ threadId: parseId(threadId) }),
    stringify: ({ threadId }) => ({ threadId: `${threadId}` }),
  },
  ...chunked(() =>
    import("./features/work/handoff-resolver.tsx").then((module) => ({
      default: module.LinksResolver,
    })),
  ),
});

/** Bare classic message links resolve their conversation before opening its permalink. */
const messageRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "m/$messageId",
  params: {
    parse: ({ messageId }) => ({ messageId: parseId(messageId) }),
    stringify: ({ messageId }) => ({ messageId: `${messageId}` }),
  },
  ...chunked(() =>
    import("./features/room/message-resolver.tsx").then((module) => ({
      default: module.MessageResolver,
    })),
  ),
});

/** Existing room controls, opened by their classic page's URL. */
const roomControlRoutes = [
  createRoute({ getParentRoute: () => roomRoute, path: "threads", component: () => null }),
  createRoute({ getParentRoute: () => roomRoute, path: "files", component: () => null }),
  createRoute({ getParentRoute: () => roomRoute, path: "pins", component: () => null }),
  createRoute({ getParentRoute: () => roomRoute, path: "automations", component: () => null }),
  createRoute({ getParentRoute: () => roomRoute, path: "notifications", component: () => null }),
  // The room's settings dialog (`RoomSettingsHost`), over the conversation.
  createRoute({ getParentRoute: () => roomRoute, path: "settings", component: () => null }),
];

/** The board owns its new-post dialog; this route opens no right pane. */
const newBoardPostRoute = createRoute({
  getParentRoute: () => roomRoute,
  path: "posts/new",
  component: () => null,
});

/** `/app/rooms/new/<kind>`: the create-a-room dialog, opened on that kind over the home screen. */
const newRoomRoutes = NEW_ROOM_SLUGS.map((kind) =>
  createRoute({
    getParentRoute: () => shellRoute,
    path: `rooms/new/${kind}`,
    component: () => <NewRoomRoute kind={kind} />,
  }),
);

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

/** `/app/r/$roomId/t/$threadId/handoff`: the thread pane with its handoff dialog open over it. */
const handoffRoute = createRoute({
  getParentRoute: () => threadRoute,
  path: "handoff",
  component: () => null,
});

/** `/app/r/$roomId/t/$threadId/links`: the thread pane with its link editor open. */
const linksRoute = createRoute({
  getParentRoute: () => threadRoute,
  path: "links",
  component: () => null,
});

/** A message's id in a "Create Fizzy card" URL (not `messageId`: that would refocus the room). */
const sourceParams = {
  parse: ({ sourceId }: { readonly sourceId: string }) => ({ sourceId: parseId(sourceId) }),
  stringify: ({ sourceId }: { readonly sourceId: number }) => ({ sourceId: `${sourceId}` }),
};

/**
 * `/app/r/$roomId/m/$sourceId/fizzy/new`: the room with "Create Fizzy card" open on a message of
 * its timeline (the classic form page); `…/t/$threadId/m/$sourceId/fizzy/new` opens it on a reply,
 * over the thread. The room draws the dialog (features/fizzy/fizzy-card-overlay.tsx).
 */
const fizzyCardRoute = createRoute({
  getParentRoute: () => roomRoute,
  path: "m/$sourceId/fizzy/new",
  params: sourceParams,
  component: () => null,
});

const threadFizzyCardRoute = createRoute({
  getParentRoute: () => threadRoute,
  path: "m/$sourceId/fizzy/new",
  params: sourceParams,
  component: () => null,
});

/** `/app/settings`: the classic profile page's sections, the profile first. */
const settingsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "settings",
  component: SettingsView,
});

/**
 * The settings sections, each at `/app/settings/<path>`. `/app/settings` shows the profile beside
 * the nav; on phones it is the list of sections (the profile kept under it, hidden), and the
 * profile is pushed at `…/profile`.
 */
const settingsSections = [
  createRoute({ getParentRoute: () => settingsRoute, path: "/", component: ProfileSection }),
  createRoute({ getParentRoute: () => settingsRoute, path: "profile", component: ProfileSection }),
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

/**
 * The admin sections, each at `/app/admin/<path>`. `/app/admin` shows the workspace beside the
 * nav; on phones it is the list of sections (the workspace kept under it, hidden), and the
 * workspace is pushed at `…/workspace`.
 */
const adminSections = [
  createRoute({ getParentRoute: () => adminRoute, path: "/", component: WorkspaceSection }),
  createRoute({ getParentRoute: () => adminRoute, path: "workspace", component: WorkspaceSection }),
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
  ...chunked(() =>
    import("./features/activity/activity-route.tsx").then((module) => ({
      default: module.ActivityRoute,
    })),
  ),
});

/** `/app/saved?status=`: saved messages (Slack's "Later"). */
const savedRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "saved",
  validateSearch: parseSavedSearch,
  ...chunked(() =>
    import("./features/saved/saved-route.tsx").then((module) => ({ default: module.SavedRoute })),
  ),
});

/** `/app/scheduled`: every scheduled message, upcoming, stranded and past. */
const scheduledRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "scheduled",
  ...chunked(() =>
    import("./features/scheduled/scheduled-page.tsx").then((module) => ({
      default: module.ScheduledPage,
    })),
  ),
});

/** `/app/work?state=`: every work thread, by tab (its own chunk). */
const workRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "work",
  validateSearch: parseWorkSearch,
  ...chunked(() =>
    import("./features/work/work-route.tsx").then((module) => ({
      default: module.WorkRoute,
    })),
  ),
});

/** `/app/agents`: every agent in the workspace, with live status (S4). */
const agentsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "agents",
  ...chunked(() =>
    import("./features/agents/agent-directory-page.tsx").then((module) => ({
      default: module.AgentDirectoryPage,
    })),
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
  ...chunked(() =>
    import("./features/agents/agent-profile-route.tsx").then((module) => ({
      default: module.AgentProfileRoute,
    })),
  ),
});

/** `/app/agents/$agentId`: the profile's overview section. */
const agentOverviewRoute = createRoute({
  getParentRoute: () => agentRoute,
  path: "/",
  ...chunked(() =>
    import("./features/agents/agent-profile-page.tsx").then((module) => ({
      default: module.AgentOverviewRoute,
    })),
  ),
});

/** `/app/agents/$agentId/approvals?status=`: an agent's approval requests (S4). */
const agentApprovalsRoute = createRoute({
  getParentRoute: () => agentRoute,
  path: "approvals",
  validateSearch: parseApprovalsSearch,
  ...chunked(() =>
    import("./features/agents/agent-approvals-tab.tsx").then((module) => ({
      default: module.AgentApprovalsRoute,
    })),
  ),
});

/** `/app/agents/$agentId/events?outcome=`: an agent's activity ledger (S4). */
const agentEventsRoute = createRoute({
  getParentRoute: () => agentRoute,
  path: "events",
  validateSearch: parseLedgerSearch,
  ...chunked(() =>
    import("./features/agents/agent-ledger-tab.tsx").then((module) => ({
      default: module.AgentLedgerRoute,
    })),
  ),
});

/** `/app/people`: the workspace's people, to message or huddle with (its own chunk). */
const peopleRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "people",
  ...chunked(() =>
    import("./features/people/people-page.tsx").then((module) => ({ default: module.PeoplePage })),
  ),
});

/** `/app/people/$userId`: someone's page, or settings for the viewer's own id. */
const personRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "people/$userId",
  params: {
    parse: ({ userId }) => ({ userId: parseId(userId) }),
    stringify: ({ userId }) => ({ userId: `${userId}` }),
  },
  ...chunked(() =>
    import("./features/people/person-page.tsx").then((module) => ({
      default: module.PersonRoute,
    })),
  ),
});

/** `/app/r/$roomId/events`: a room's calendar (its own chunk); `…/new` opens the form over it. */
const eventsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "r/$roomId/events",
  params: {
    parse: ({ roomId }) => ({ roomId: parseId(roomId) }),
    stringify: ({ roomId }) => ({ roomId: `${roomId}` }),
  },
  ...chunked(() =>
    import("./features/events/events-page.tsx").then((module) => ({
      default: module.EventsRoute,
    })),
  ),
});

const newEventRoute = createRoute({
  getParentRoute: () => eventsRoute,
  path: "new",
  component: () => null,
});

/**
 * `/app/r/$roomId/events/$eventId`: an event's page; `…/edit` opens the form over it and
 * `…/attendance` opens it on the viewer's response.
 */
const eventRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "r/$roomId/events/$eventId",
  params: {
    parse: ({ roomId, eventId }) => ({ roomId: parseId(roomId), eventId: parseId(eventId) }),
    stringify: ({ roomId, eventId }) => ({ roomId: `${roomId}`, eventId: `${eventId}` }),
  },
  ...chunked(() =>
    import("./features/events/event-page.tsx").then((module) => ({
      default: module.EventRoute,
    })),
  ),
});

const eventChildRoutes = [
  createRoute({ getParentRoute: () => eventRoute, path: "edit", component: () => null }),
  createRoute({ getParentRoute: () => eventRoute, path: "attendance", component: () => null }),
];

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
  ...chunked(() =>
    import("./features/search/search-page.tsx").then((module) => ({ default: module.SearchPage })),
  ),
});

const routeTree = rootRoute.addChildren([
  kitchenSinkRoute,
  shellRoute.addChildren([
    homeRoute,
    activityRoute,
    savedRoute,
    scheduledRoute,
    workRoute,
    agentsRoute,
    agentRoute.addChildren([agentOverviewRoute, agentApprovalsRoute, agentEventsRoute]),
    searchRoute,
    peopleRoute,
    personRoute,
    messageRoute,
    handoffResolverRoute,
    linksResolverRoute,
    ...newRoomRoutes,
    roomRoute.addChildren([
      permalinkRoute,
      fizzyCardRoute,
      newThreadRoute,
      newBoardPostRoute,
      threadRoute.addChildren([threadFizzyCardRoute, handoffRoute, linksRoute]),
      ...roomControlRoutes,
    ]),
    eventsRoute.addChildren([newEventRoute]),
    eventRoute.addChildren(eventChildRoutes),
    settingsRoute.addChildren(settingsSections),
    adminRoute.addChildren(adminSections),
  ]),
]);

export const router = createRouter({
  routeTree,
  basepath: import.meta.env.BASE_URL,
  defaultPreload: false,
  // Loaders fetch the chunk. Until `pendingMs`, the current screen stays; Suspense then hides
  // only the incoming pane. Hover preloads still warm the module cache.
  defaultPendingMs: ROUTE_PENDING_DELAY_MS,
  // 0, not TanStack's 500ms, including reduced motion: the 150ms wait and the ~300ms reveal already stop a flash.
  defaultPendingMinMs: ROUTE_PENDING_MIN_MS,
  defaultPendingComponent: RoutePending,
  // A destination the SPA hasn't ported yet opens on its classic page (src/lib/screens.ts).
  defaultNotFoundComponent: NotFound,
  scrollRestoration: false,
});

captureInitialMessageLink(router.history);

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
