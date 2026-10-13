import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import type { Grant } from "../../gen/Grant.ts";
import type { GrantList } from "../../gen/GrantList.ts";
import { bots } from "../../sync/admin.ts";
import { BotGrantsSection } from "./bot-grants-section.tsx";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

async function renderSection() {
  const root = createRootRoute();

  const section = createRoute({
    getParentRoute: () => root,
    path: "admin/bots/$botId/grants",
    component: BotGrantsSection,
  });

  const router = createRouter({
    routeTree: root.addChildren([section]),
    basepath: "/app",
    history: createMemoryHistory({ initialEntries: ["/app/admin/bots/1/grants"] }),
    defaultNotFoundComponent: () => null,
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());
}

const legacy: Grant = {
  id: 7,
  capability: "external_action",
  roomName: "Workspace-wide",
  grantedBy: "David",
  createdAt: "2026-03-02T16:00:00Z",
  revoked: false,
  status: "incompatible",
};

const list: GrantList = {
  botId: 1,
  botName: "Personal bot",
  canGrant: false,
  legacy: false,
  grants: [legacy],
  capabilities: [],
  rooms: [],
};

it("shows incompatible grants as inactive and keeps their audit record and revoke action", async () => {
  vi.spyOn(bots, "grants").mockResolvedValue(list);

  const revoke = vi.spyOn(bots, "revokeGrant").mockResolvedValue({
    ...list,
    grants: [{ ...legacy, revoked: true, status: "revoked" }],
  });

  await renderSection();
  const label = await screen.findByText("external_action");
  const row = label.closest("li");

  if (row === null) throw new Error("No grant row");

  expect(within(row).getByText("Inactive · incompatible with personal agent policy")).toBeTruthy();
  expect(within(row).getByText(/By David,/)).toBeTruthy();
  fireEvent.click(
    within(row).getByRole("button", { name: /^Revoke\s*external_action \(Workspace-wide\)$/ }),
  );
  expect(await within(row).findByText("Revoked")).toBeTruthy();
  expect(revoke).toHaveBeenCalledWith(1, 7);
  expect(within(row).queryByRole("button")).toBeNull();
  expect(within(row).getByText("external_action")).toBeTruthy();
});
