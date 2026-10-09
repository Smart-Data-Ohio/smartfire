import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { mutations, store } from "../../store/store.ts";
import { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { HandoffDialog, useHandoffRoute } from "./handoff-dialog.tsx";
import {
  factsFixture,
  threadDetailFixture,
  workDetailFixture,
  workPermissionsFixture,
} from "./test-fixtures.ts";

const THREAD = 7;

function loadThread() {
  mutations.loadThreadDetail(
    threadDetailFixture(THREAD, factsFixture(), workDetailFixture(), workPermissionsFixture()),
  );
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

  const handoff = createRoute({
    getParentRoute: () => thread,
    path: "handoff",
    component: () => null,
  });

  const earlier = createRoute({
    getParentRoute: () => root,
    path: "/earlier",
    component: () => <p>Earlier page</p>,
  });

  const router = createRouter({
    routeTree: root.addChildren([earlier, thread.addChildren([handoff])]),
    history: createMemoryHistory({ initialEntries: ["/earlier", path] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());

  return router;
}

/** The work bar's wiring: the button pushes the dialog, and both closes use the same route. */
function RouteHarness() {
  const route = useHandoffRoute(THREAD);
  const work = store.getState().work.details[THREAD];

  return (
    <>
      <button type="button" onClick={route.openHandoff}>
        Open handoff
      </button>
      {work === undefined ? null : (
        <HandoffDialog
          threadId={THREAD}
          threadName="Cursor pagination"
          work={work}
          open={route.open}
          onOpenChange={(next) => {
            if (!next) {
              route.closeHandoff();
            }
          }}
        />
      )}
    </>
  );
}

beforeEach(() => {
  // jsdom has no matchMedia; the agent avatar asks it which theme is on screen.
  window.matchMedia = (query: string) =>
    Object.assign(new EventTarget(), {
      matches: false,
      media: query,
      onchange: null,
      addListener: () => undefined,
      removeListener: () => undefined,
    });
});

afterEach(() => {
  vi.restoreAllMocks();
  mutations.reset();
});

describe("the handoff route", () => {
  it("steps back to the prior entry when an in-app open is cancelled", async () => {
    const router = await mount(`/r/4/t/${THREAD}`);
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "Open handoff" }));
    await screen.findByRole("dialog", { name: /Hand off/ });
    expect(router.history.location.pathname).toBe(`/r/4/t/${THREAD}/handoff`);

    await user.click(screen.getByRole("button", { name: "Cancel" }));

    await waitFor(() => expect(router.history.location.pathname).toBe(`/r/4/t/${THREAD}`));
    await act(async () => router.history.back());
    expect(router.history.location.pathname).toBe("/earlier");
  });

  it("replaces a direct arrival with the thread when it closes", async () => {
    const router = await mount(`/r/4/t/${THREAD}/handoff`);
    const user = userEvent.setup();

    await screen.findByRole("dialog", { name: /Hand off/ });
    expect(router.history.length).toBe(2);

    await user.click(screen.getByRole("button", { name: "Cancel" }));

    await waitFor(() => expect(router.history.location.pathname).toBe(`/r/4/t/${THREAD}`));
    expect(router.history.length).toBe(2);
  });

  it("steps back after a successful in-app handoff, the same way a cancel does", async () => {
    vi.spyOn(actions.work, "handOff").mockResolvedValue(
      threadDetailFixture(THREAD, factsFixture(), workDetailFixture()),
    );

    const router = await mount(`/r/4/t/${THREAD}`);
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "Open handoff" }));

    const dialog = await screen.findByRole("dialog", { name: /Hand off/ });

    await user.type(within(dialog).getByRole("textbox", { name: "Summary" }), "Over to you");
    await user.click(within(dialog).getByRole("button", { name: "Hand off" }));

    await waitFor(() => expect(router.history.location.pathname).toBe(`/r/4/t/${THREAD}`));
    await act(async () => router.history.back());
    expect(router.history.location.pathname).toBe("/earlier");
  });
});

describe("the handoff form's server errors", () => {
  it("shows a validation error on its field", async () => {
    vi.spyOn(actions.work, "handOff").mockRejectedValue(
      new ActionError("Validation", "Summary is too long (maximum is 2000 characters)", {
        summary: ["is too long (maximum is 2000 characters)"],
        receiverAgentId: ["Receiver must hold the manage_threads capability in this room"],
      }),
    );

    loadThread();

    const work = store.getState().work.details[THREAD];

    if (work === undefined) {
      throw new Error("expected the work detail");
    }

    const user = userEvent.setup();

    render(
      <HandoffDialog
        threadId={THREAD}
        threadName="Cursor pagination"
        work={work}
        open
        onOpenChange={() => undefined}
      />,
    );

    const dialog = await screen.findByRole("dialog", { name: /Hand off/ });

    await user.click(within(dialog).getByRole("radio", { name: /Ember/ }));
    await user.type(within(dialog).getByRole("textbox", { name: "Summary" }), "Over to you");
    await user.click(within(dialog).getByRole("button", { name: "Hand off" }));

    expect(
      await within(dialog).findByText("Summary is too long (maximum is 2000 characters)."),
    ).toBeTruthy();
    expect(
      within(dialog).getByText("Receiver must hold the manage_threads capability in this room."),
    ).toBeTruthy();
    expect(within(dialog).queryByRole("alert")).toBeNull();

    await user.type(within(dialog).getByRole("textbox", { name: "Summary" }), "!");
    expect(
      within(dialog).queryByText("Summary is too long (maximum is 2000 characters)."),
    ).toBeNull();
  });
});
