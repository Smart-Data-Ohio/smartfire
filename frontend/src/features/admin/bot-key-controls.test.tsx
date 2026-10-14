import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, cleanup, render, screen, within } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, expect, it, vi } from "vitest";
import type { Bot } from "../../gen/Bot.ts";
import type { Workspace } from "../../gen/Workspace.ts";
import { bots, oauthClients } from "../../sync/admin.ts";
import { AdminContext } from "./admin-parts.tsx";
import { BotSection } from "./bot-section.tsx";
import { BotsSection } from "./bots-section.tsx";

const workspace: Workspace = {
  name: "Smartfire",
  logoUrl: "/logo",
  logoStillUrl: null,
  bannerUrl: null,
  bannerStillUrl: null,
  logoAttached: false,
  inviteSplashUrl: null,
  joinUrl: "https://chat.example/join/test",
  canAdminister: true,
  restrictRoomCreationToAdministrators: false,
  afkPolicy: { roomId: null, timeoutMinutes: 5 },
  uploadLimitBytes: 100 * 1024 * 1024,
  stageAudienceBudget: 300,
  memberAgentsEnabled: true,
  memberAgentLimit: 3,
  version: "test",
  description: "",
  vanitySlug: null,
  vanityUrl: null,
};

const bot = {
  id: 9,
  name: "Personal helper",
  avatarUrl: "/avatar.svg",
  avatarAttached: false,
  iconName: null,
  icon: null,
  webhookUrl: null,
  canAdminister: true,
  supportsBotKey: false,
  agent: {
    id: 11,
    provider: null,
    runtime: null,
    description: null,
    dailyMessageCap: null,
    dailyBoardPostCap: null,
    dailyExternalActionCap: null,
    usage: "0 messages",
    suspended: false,
    ledgerUrl: "/agents/11/events",
    approvalsUrl: "/agents/11/approvals",
  },
  signingSecret: null,
  github: null,
  botAvatar: null,
} satisfies Bot;

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

async function renderSection(ui: ReactNode, detail = false) {
  const root = createRootRoute({
    component: () => <AdminContext value={{ workspace, replace: () => {} }}>{ui}</AdminContext>,
  });

  const section = createRoute({
    getParentRoute: () => root,
    path: detail ? "admin/bots/$botId" : "admin/bots",
  });

  const router = createRouter({
    routeTree: root.addChildren([section]),
    basepath: "/app",
    history: createMemoryHistory({
      initialEntries: [detail ? "/app/admin/bots/9" : "/app/admin/bots"],
    }),
    defaultNotFoundComponent: () => null,
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());
}

it("keeps personal-agent management without offering an unavailable bot key", async () => {
  vi.spyOn(bots, "get").mockResolvedValue(bot);
  await renderSection(<BotSection />, true);
  expect(await screen.findByRole("link", { name: "Credentials" })).toBeTruthy();
  expect(screen.getByRole("link", { name: "Grants" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Kill switch" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Remove this bot" })).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Generate a new key" })).toBeNull();
});

it("keeps workspace bot-key generation available", async () => {
  vi.spyOn(bots, "get").mockResolvedValue({ ...bot, supportsBotKey: true });
  await renderSection(<BotSection />, true);
  expect(await screen.findByRole("button", { name: "Generate a new key" })).toBeTruthy();
});

it("shows bot-key posting commands only for bots that support keys", async () => {
  const summary = {
    id: bot.id,
    name: bot.name,
    avatarUrl: bot.avatarUrl,
    icon: null,
    botAvatar: null,
    ownership: "Personal agent · Owned by David",
    supportsBotKey: false,
    rooms: [
      {
        id: 1,
        name: "Ops",
        messageCommand: "curl -d 'Hello!' /rooms/1/BOT_KEY/messages",
        attachmentCommand: 'curl -F "attachment=@/path/to/file" /rooms/1/BOT_KEY/messages',
      },
    ],
  };

  vi.spyOn(bots, "list").mockResolvedValue({
    bots: [summary, { ...summary, id: 10, name: "Workspace helper", supportsBotKey: true }],
  });
  vi.spyOn(oauthClients, "list").mockResolvedValue({ clients: [], oauthAvailable: false });
  await renderSection(<BotsSection />);

  const personal = (await screen.findByText("Personal helper", { selector: "strong" })).closest(
    "li",
  );

  const workspaceBot = screen.getByText("Workspace helper", { selector: "strong" }).closest("li");

  if (personal === null || workspaceBot === null) throw new Error("Missing bot row");
  expect(within(personal).getByText("Edit", { selector: "a" })).toBeTruthy();
  expect(personal.querySelector("details")).toBeNull();
  expect(workspaceBot.querySelector("details")).toBeTruthy();
  expect(within(workspaceBot).getByText("curl -d 'Hello!' /rooms/1/BOT_KEY/messages")).toBeTruthy();
});
