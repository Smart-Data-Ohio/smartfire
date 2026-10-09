import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { HandoffResolver, LinksResolver } from "./handoff-resolver.tsx";
import {
  factsFixture,
  threadDetailFixture,
  workDetailFixture,
  workPermissionsFixture,
} from "./test-fixtures.ts";

async function mount(path: string) {
  const root = createRootRoute({ component: Outlet });

  const resolver = createRoute({
    getParentRoute: () => root,
    path: "/t/$threadId/handoff",
    params: {
      parse: ({ threadId }) => ({ threadId: Number(threadId) }),
      stringify: ({ threadId }) => ({ threadId: String(threadId) }),
    },
    component: HandoffResolver,
  });

  const thread = createRoute({
    getParentRoute: () => root,
    path: "/r/$roomId/t/$threadId",
    component: () => (
      <>
        <p>Thread destination</p>
        <Outlet />
      </>
    ),
  });

  const handoff = createRoute({
    getParentRoute: () => thread,
    path: "handoff",
    component: () => <p>Handoff destination</p>,
  });

  const linksResolver = createRoute({
    getParentRoute: () => root,
    path: "/t/$threadId/links",
    params: {
      parse: ({ threadId }) => ({ threadId: Number(threadId) }),
      stringify: ({ threadId }) => ({ threadId: String(threadId) }),
    },
    component: LinksResolver,
  });

  const links = createRoute({
    getParentRoute: () => thread,
    path: "links",
    component: () => <p>Links destination</p>,
  });

  const router = createRouter({
    routeTree: root.addChildren([resolver, linksResolver, thread.addChildren([handoff, links])]),
    basepath: "/app",
    history: createMemoryHistory({ initialEntries: [path] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());

  return router;
}

afterEach(() => vi.restoreAllMocks());

describe("HandoffResolver", () => {
  it("finds the thread's room and replaces itself with the thread's handoff dialog", async () => {
    const read = vi
      .spyOn(actions.threads, "read")
      .mockResolvedValue(threadDetailFixture(7, factsFixture(), workDetailFixture()));

    const router = await mount("/app/t/7/handoff");

    await screen.findByText("Handoff destination");
    expect(read).toHaveBeenCalledWith(7);
    expect(router.history.location.href).toBe("/app/r/4/t/7/handoff");
    expect(router.history.length).toBe(1);
  });

  it("shows the unavailable page for a thread the viewer can't see", async () => {
    vi.spyOn(actions.threads, "read").mockRejectedValue(new ActionError("NotFound", "Not found"));
    const router = await mount("/app/t/7/handoff");

    await screen.findByRole("region", { name: "Page not found" });
    expect(router.history.location.href).toBe("/app/t/7/handoff");
  });

  it("explains an untracked thread and does not open the dialog", async () => {
    vi.spyOn(actions.threads, "read").mockResolvedValue(threadDetailFixture(7, null, null));
    const router = await mount("/app/t/7/handoff");

    expect((await screen.findByRole("alert")).textContent).toBe(
      "This thread isn't tracked as work",
    );
    expect(screen.queryByText("Handoff destination")).toBeNull();
    expect(router.history.location.href).toBe("/app/t/7/handoff");
    expect(router.history.length).toBe(1);
  });

  it("explains a thread the viewer can't manage and does not open the dialog", async () => {
    vi.spyOn(actions.threads, "read").mockResolvedValue(
      threadDetailFixture(
        7,
        factsFixture(),
        workDetailFixture(),
        workPermissionsFixture({ canManageWork: false }),
      ),
    );
    const router = await mount("/app/t/7/handoff");

    expect((await screen.findByRole("alert")).textContent).toBe(
      "You cannot manage work in this thread",
    );
    expect(screen.queryByText("Handoff destination")).toBeNull();
    expect(router.history.location.href).toBe("/app/t/7/handoff");
    expect(router.history.length).toBe(1);
  });

  it("explains a thread no agent can take, and does not open the dialog", async () => {
    vi.spyOn(actions.threads, "read").mockResolvedValue(
      threadDetailFixture(7, factsFixture(), workDetailFixture({ handoffReceivers: [] })),
    );
    const router = await mount("/app/t/7/handoff");

    expect((await screen.findByRole("alert")).textContent).toBe(
      "No agent here can take this work. An agent needs to be in this room and allowed to post, manage threads and read messages.",
    );
    expect(screen.queryByText("Handoff destination")).toBeNull();
    expect(router.history.location.href).toBe("/app/t/7/handoff");
    expect(router.history.length).toBe(1);
  });

  it("finds the thread's room and replaces itself with the post's link form", async () => {
    const locate = vi.spyOn(actions.threads, "locate").mockResolvedValue(4);
    const router = await mount("/app/t/7/links");

    await screen.findByText("Links destination");
    expect(locate).toHaveBeenCalledWith(7);
    expect(router.history.location.href).toBe("/app/r/4/t/7/links");
    expect(router.history.length).toBe(1);
  });
});
