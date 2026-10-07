import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { openPane } from "./pane-store.ts";
import { usePaneNavigation } from "./use-right-pane.ts";

function Probe() {
  const navigation = usePaneNavigation(4);

  return (
    <>
      <output>{JSON.stringify(navigation.view)}</output>
      <button type="button" onClick={navigation.closeTop}>
        Back
      </button>
      <button type="button" onClick={navigation.closeAll}>
        Close
      </button>
      <button type="button" onClick={() => navigation.toggle("files")}>
        Files
      </button>
      <button type="button" onClick={() => navigation.toggle("members")}>
        Members
      </button>
      <button type="button" onClick={() => navigation.openThread(7)}>
        Thread
      </button>
    </>
  );
}

async function mount(path: string) {
  const root = createRootRoute({ component: Probe });
  const room = createRoute({ getParentRoute: () => root, path: "/r/$roomId" });
  const controls = ["threads", "files", "pins"].map((pane) =>
    createRoute({ getParentRoute: () => room, path: pane }),
  );
  const thread = createRoute({
    getParentRoute: () => room,
    path: "t/$threadId",
    params: {
      parse: ({ threadId }) => ({ threadId: Number(threadId) }),
      stringify: ({ threadId }) => ({ threadId: String(threadId) }),
    },
  });
  const draft = createRoute({ getParentRoute: () => room, path: "t/new" });
  const router = createRouter({
    routeTree: root.addChildren([room.addChildren([...controls, thread, draft])]),
    history: createMemoryHistory({ initialEntries: [path] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());

  return router;
}

afterEach(() => openPane(null));

describe("URL pane navigation", () => {
  it("opens the mapped pane and closes both the pane and its URL", async () => {
    const router = await mount("/r/4/files");
    const user = userEvent.setup();

    expect(screen.getByRole("status").textContent).toBe('{"kind":"pane","pane":"files"}');
    await user.click(screen.getByRole("button", { name: "Close" }));
    await waitFor(() => expect(router.state.location.pathname).toBe("/r/4"));
    expect(screen.getByRole("status").textContent).toBe("null");
  });

  it("keeps header changes in the URL, and leaves it for a local-only pane", async () => {
    const router = await mount("/r/4/pins");
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "Files" }));
    await waitFor(() => expect(router.state.location.pathname).toBe("/r/4/files"));
    await user.click(screen.getByRole("button", { name: "Members" }));
    await waitFor(() => expect(router.state.location.pathname).toBe("/r/4"));
    expect(screen.getByRole("status").textContent).toBe('{"kind":"pane","pane":"members"}');
  });

  it("returns from a thread to the routed list, then closes that list's URL", async () => {
    const router = await mount("/r/4/threads");
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "Thread" }));
    await waitFor(() => expect(router.state.location.pathname).toBe("/r/4/t/7"));
    await user.click(screen.getByRole("button", { name: "Back" }));
    await waitFor(() => expect(router.state.location.pathname).toBe("/r/4/threads"));
    await user.click(screen.getByRole("button", { name: "Back" }));
    await waitFor(() => expect(router.state.location.pathname).toBe("/r/4"));
    expect(screen.getByRole("status").textContent).toBe("null");
  });

  it("retains the existing local pane behavior on a base room URL", async () => {
    const router = await mount("/r/4");
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "Files" }));
    expect(router.state.location.pathname).toBe("/r/4");
    await user.click(screen.getByRole("button", { name: "Thread" }));
    await waitFor(() => expect(router.state.location.pathname).toBe("/r/4/t/7"));
    await user.click(screen.getByRole("button", { name: "Back" }));
    await waitFor(() => expect(router.state.location.pathname).toBe("/r/4"));
    expect(screen.getByRole("status").textContent).toBe('{"kind":"pane","pane":"files"}');
  });
});
