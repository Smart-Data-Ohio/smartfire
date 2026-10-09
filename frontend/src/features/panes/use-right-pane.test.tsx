import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
  useParams,
} from "@tanstack/react-router";
import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { roomDetailFixture } from "../../api/testing.ts";
import { parseBoardSearch } from "../../lib/board-search.ts";
import { beginRoomRequest } from "../../store/join-state.ts";
import { mutations } from "../../store/store.ts";
import { openPane } from "./pane-store.ts";
import { usePaneNavigation, useRoomPaneLifecycle } from "./use-right-pane.ts";

function ThreadListProbe({ roomId }: { readonly roomId: number }) {
  const navigation = usePaneNavigation(roomId);

  return (
    <button type="button" onClick={() => navigation.openThread(7)}>
      Listed thread
    </button>
  );
}

function Probe() {
  const { roomId } = useParams({ strict: false });

  useRoomPaneLifecycle(roomId ?? 0);

  const navigation = usePaneNavigation(roomId ?? 0);

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
      <button type="button" onClick={() => navigation.toggle("automations")}>
        Automations
      </button>
      <button type="button" onClick={() => navigation.openThread(7)}>
        Thread
      </button>
      <button type="button" onClick={() => navigation.toggle("details")}>
        Details
      </button>
      <button type="button" onClick={() => navigation.push("files")}>
        Push files
      </button>
      {navigation.view?.kind === "pane" && navigation.view.pane === "threads" ? (
        <ThreadListProbe roomId={roomId ?? 0} />
      ) : null}
    </>
  );
}

async function mount(path: string) {
  const root = createRootRoute({ component: Probe });

  const room = createRoute({
    getParentRoute: () => root,
    path: "/r/$roomId",
    validateSearch: parseBoardSearch,
    params: {
      parse: ({ roomId }) => ({ roomId: Number(roomId) }),
      stringify: ({ roomId }) => ({ roomId: String(roomId) }),
    },
  });

  const controls = ["threads", "files", "pins", "automations"].map((pane) =>
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

  const notifications = createRoute({ getParentRoute: () => room, path: "notifications" });

  const router = createRouter({
    routeTree: root.addChildren([room.addChildren([...controls, thread, draft, notifications])]),
    history: createMemoryHistory({ initialEntries: [path] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());

  return router;
}

afterEach(() => {
  openPane(null);
  mutations.reset();
});

describe("URL pane navigation", () => {
  it("opens automations by URL from the base board, retains filters and returns after a thread", async () => {
    const detail = roomDetailFixture(900);
    detail.room.kind = "board";
    mutations.setRoomDetail(detail, beginRoomRequest());
    const router = await mount("/r/900?view=board&status=all&owner=me&tag=api");
    const user = userEvent.setup();
    const filters = { view: "board", status: "all", owner: "me", tag: "api" };
    await user.click(screen.getByRole("button", { name: "Automations" }));
    await waitFor(() => expect(router.state.location.pathname).toBe("/r/900/automations"));
    expect(screen.getByRole("status").textContent).toBe('{"kind":"pane","pane":"automations"}');
    expect(router.state.location.search).toEqual(filters);
    await user.click(screen.getByRole("button", { name: "Thread" }));
    await waitFor(() => expect(router.state.location.pathname).toBe("/r/900/t/7"));
    expect(router.state.location.search).toEqual(filters);
    await user.click(screen.getByRole("button", { name: "Back" }));
    await waitFor(() => expect(router.state.location.pathname).toBe("/r/900/automations"));
    expect(router.state.location.search).toEqual(filters);
    await user.click(screen.getByRole("button", { name: "Automations" }));
    await waitFor(() => expect(router.state.location.pathname).toBe("/r/900"));
    expect(router.state.location.search).toEqual(filters);
    expect(screen.getByRole("status").textContent).toBe("null");
  });

  it("opens a direct automations URL and closes it while retaining board filters", async () => {
    const detail = roomDetailFixture(900);
    detail.room.kind = "board";
    mutations.setRoomDetail(detail, beginRoomRequest());
    const router = await mount("/r/900/automations?view=list&status=done&owner=7&tag=design");
    const user = userEvent.setup();
    expect(screen.getByRole("status").textContent).toBe('{"kind":"pane","pane":"automations"}');
    await user.click(screen.getByRole("button", { name: "Close" }));
    await waitFor(() => expect(router.state.location.pathname).toBe("/r/900"));
    expect(router.state.location.search).toEqual({
      view: "list",
      status: "done",
      owner: "7",
      tag: "design",
    });
    expect(screen.getByRole("status").textContent).toBe("null");
  });

  it("opens an automations URL on a non-board room so the pane can explain", async () => {
    mutations.setRoomDetail(roomDetailFixture(4), beginRoomRequest());
    openPane("files");
    const router = await mount("/r/4/automations");
    expect(router.state.location.pathname).toBe("/r/4/automations");
    expect(screen.getByRole("status").textContent).toBe('{"kind":"pane","pane":"automations"}');
  });

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

  it("keeps a routed return list after its nested navigation caller unmounts", async () => {
    const router = await mount("/r/4/threads");
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "Listed thread" }));
    await waitFor(() => expect(router.state.location.pathname).toBe("/r/4/t/7"));
    expect(screen.queryByRole("button", { name: "Listed thread" })).toBeNull();
    await user.click(screen.getByRole("button", { name: "Back" }));
    await waitFor(() => expect(router.state.location.pathname).toBe("/r/4/threads"));
    expect(screen.getByRole("button", { name: "Listed thread" })).toBeTruthy();
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

  it("clears a routed list on browser Back to the base room", async () => {
    const router = await mount("/r/4");

    await act(() => router.navigate({ href: "/r/4/files" }));
    expect(screen.getByRole("status").textContent).toBe('{"kind":"pane","pane":"files"}');
    await act(async () => router.history.back());
    await waitFor(() => expect(router.state.location.pathname).toBe("/r/4"));
    expect(screen.getByRole("status").textContent).toBe("null");
  });

  it("forgets a thread's routed return list after visiting another room", async () => {
    const router = await mount("/r/4/threads");
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "Thread" }));
    await waitFor(() => expect(router.state.location.pathname).toBe("/r/4/t/7"));
    await act(() => router.navigate({ href: "/r/5/t/8" }));
    await act(() => router.navigate({ href: "/r/4/t/7" }));
    await user.click(screen.getByRole("button", { name: "Back" }));
    await waitFor(() => expect(router.state.location.pathname).toBe("/r/4"));
    expect(screen.getByRole("status").textContent).toBe("null");
  });

  it("closes a local pane opened after leaving the notification URL when Back returns there", async () => {
    const router = await mount("/r/4/notifications");
    const user = userEvent.setup();

    await act(() => router.navigate({ href: "/r/4" }));
    await user.click(screen.getByRole("button", { name: "Members" }));
    expect(screen.getByRole("status").textContent).toBe('{"kind":"pane","pane":"members"}');
    await act(async () => router.history.back());
    await waitFor(() => expect(router.state.location.pathname).toBe("/r/4/notifications"));
    await waitFor(() => expect(screen.getByRole("status").textContent).toBe("null"));
  });

  it("pushes a pane over the details, and Back returns to them before closing", async () => {
    const router = await mount("/r/4");
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "Details" }));
    await user.click(screen.getByRole("button", { name: "Push files" }));
    expect(router.state.location.pathname).toBe("/r/4");
    expect(screen.getByRole("status").textContent).toBe('{"kind":"pane","pane":"files"}');
    await user.click(screen.getByRole("button", { name: "Back" }));
    expect(screen.getByRole("status").textContent).toBe('{"kind":"pane","pane":"details"}');
    await user.click(screen.getByRole("button", { name: "Back" }));
    expect(screen.getByRole("status").textContent).toBe("null");
  });

  it("pushes a pane from a routed list without keeping the list to return to", async () => {
    const router = await mount("/r/4/pins");
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "Push files" }));
    await waitFor(() => expect(router.state.location.pathname).toBe("/r/4"));
    expect(screen.getByRole("status").textContent).toBe('{"kind":"pane","pane":"files"}');
  });
});
