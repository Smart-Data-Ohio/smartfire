import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { mutations } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { PostLinks } from "../boards/post-links.tsx";
import { useLinksRoute } from "./links-route.ts";
import { factsFixture, threadDetailFixture } from "./test-fixtures.ts";

const THREAD = 7;

function loadThread() {
  mutations.loadThreadDetail(threadDetailFixture(THREAD, factsFixture(), null));
}

async function mount(path: string) {
  loadThread();

  const root = createRootRoute({ component: Outlet });

  const thread = createRoute({
    getParentRoute: () => root,
    path: "/r/$roomId/t/$threadId",
    component: () => (
      <>
        <RouteHarness />
        <Outlet />
      </>
    ),
  });

  const links = createRoute({
    getParentRoute: () => thread,
    path: "links",
    component: () => null,
  });

  const earlier = createRoute({
    getParentRoute: () => root,
    path: "/earlier",
    component: () => <p>Earlier page</p>,
  });

  const router = createRouter({
    routeTree: root.addChildren([earlier, thread.addChildren([links])]),
    history: createMemoryHistory({ initialEntries: ["/earlier", path] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());

  return router;
}

/** The work bar's wiring, with the post's own Link control beside it. Both opens use the route. */
function RouteHarness() {
  const route = useLinksRoute(THREAD);

  return (
    <>
      <button type="button" onClick={route.openLinks}>
        Open links
      </button>
      {route.roomId === null ? null : (
        <PostLinks threadId={THREAD} roomId={route.roomId} links={[]} editable />
      )}
    </>
  );
}

beforeEach(() => {
  vi.spyOn(actions.work, "linkForm").mockResolvedValue({ events: [] });
});

afterEach(() => {
  vi.restoreAllMocks();
  mutations.reset();
});

describe("the links route", () => {
  it("steps back to the prior entry when an in-app open is cancelled", async () => {
    const router = await mount(`/r/4/t/${THREAD}`);
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "Open links" }));
    expect(await screen.findByRole("form", { name: "Link to this work" })).toBeDefined();
    expect(router.history.location.pathname).toBe(`/r/4/t/${THREAD}/links`);

    await user.click(screen.getByRole("button", { name: "Cancel" }));

    await waitFor(() => expect(router.history.location.pathname).toBe(`/r/4/t/${THREAD}`));
    await act(async () => router.history.back());
    expect(router.history.location.pathname).toBe("/earlier");
  });

  it("steps back when a post's Link control opened the editor and it closes", async () => {
    const router = await mount(`/r/4/t/${THREAD}`);
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "Link" }));
    expect(await screen.findByRole("form", { name: "Link to this work" })).toBeDefined();
    expect(router.history.location.pathname).toBe(`/r/4/t/${THREAD}/links`);

    await user.click(screen.getByRole("button", { name: "Cancel" }));

    await waitFor(() => expect(router.history.location.pathname).toBe(`/r/4/t/${THREAD}`));
    await act(async () => router.history.back());
    expect(router.history.location.pathname).toBe("/earlier");
  });

  it("replaces a direct arrival with the thread when it closes", async () => {
    const router = await mount(`/r/4/t/${THREAD}/links`);
    const user = userEvent.setup();

    expect(screen.getByRole("form", { name: "Link to this work" })).toBeDefined();
    expect(router.history.length).toBe(2);

    await user.click(screen.getByRole("button", { name: "Cancel" }));

    await waitFor(() => expect(router.history.location.pathname).toBe(`/r/4/t/${THREAD}`));
    expect(router.history.length).toBe(2);
  });

  it("steps back after a successful in-app link, the same way a cancel does", async () => {
    vi.spyOn(actions.work, "addLink").mockResolvedValue(
      threadDetailFixture(THREAD, factsFixture(), null),
    );

    const router = await mount(`/r/4/t/${THREAD}`);
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "Open links" }));
    expect(await screen.findByRole("form", { name: "Link to this work" })).toBeDefined();
    await user.type(
      screen.getByLabelText("Pull request URL"),
      "https://github.com/acme/api/pull/13",
    );
    await user.click(screen.getByRole("button", { name: "Link pull request" }));

    await waitFor(() => expect(router.history.location.pathname).toBe(`/r/4/t/${THREAD}`));
    await act(async () => router.history.back());
    expect(router.history.location.pathname).toBe("/earlier");
  });

  it("ignores a link that succeeds after the editor was dismissed", async () => {
    const pending = Promise.withResolvers<ReturnType<typeof threadDetailFixture>>();

    vi.spyOn(actions.work, "addLink").mockReturnValue(pending.promise);

    const router = await mount(`/r/4/t/${THREAD}`);
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "Open links" }));
    expect(await screen.findByRole("form", { name: "Link to this work" })).toBeDefined();
    await user.type(
      screen.getByLabelText("Pull request URL"),
      "https://github.com/acme/api/pull/13",
    );
    await user.click(screen.getByRole("button", { name: "Link pull request" }));

    await act(async () => router.history.back());
    await waitFor(() => expect(router.history.location.pathname).toBe(`/r/4/t/${THREAD}`));

    await act(async () => {
      pending.resolve(threadDetailFixture(THREAD, factsFixture(), null));
      await pending.promise;
    });

    expect(router.history.location.pathname).toBe(`/r/4/t/${THREAD}`);
    await act(async () => router.history.back());
    expect(router.history.location.pathname).toBe("/earlier");
  });
});
