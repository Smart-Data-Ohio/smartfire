import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { MessageRead } from "../../gen/MessageRead.ts";
import { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { messageFixture } from "../threads/test-fixtures.ts";
import { captureInitialMessageLink } from "./message-link.ts";
import { MessageResolver } from "./message-resolver.tsx";

function messageRead(id: number, threadId: number | null = null): MessageRead {
  return {
    message: messageFixture(id, { threadId }),
    users: [],
    conversation: {
      roomId: 4,
      threadId,
      roomKind: "open",
      roomName: "general",
      roomIconName: null,
      threadName: threadId === null ? null : "A thread",
    },
    saved: null,
  };
}

async function mount(path: string) {
  const root = createRootRoute({ component: Outlet });

  const message = createRoute({
    getParentRoute: () => root,
    path: "/m/$messageId",
    params: {
      parse: ({ messageId }) => ({ messageId: Number(messageId) }),
      stringify: ({ messageId }) => ({ messageId: String(messageId) }),
    },
    component: MessageResolver,
  });

  const room = createRoute({
    getParentRoute: () => root,
    path: "/r/$roomId/m/$messageId",
    component: () => <p>Room destination</p>,
  });

  const thread = createRoute({
    getParentRoute: () => root,
    path: "/r/$roomId/t/$threadId",
    component: () => <p>Thread destination</p>,
  });

  const home = createRoute({
    getParentRoute: () => root,
    path: "/",
    component: () => <p>Home destination</p>,
  });

  const history = createMemoryHistory({ initialEntries: ["/app/", path] });

  const router = createRouter({
    routeTree: root.addChildren([message, room, thread, home]),
    basepath: "/app",
    history,
  });

  captureInitialMessageLink(history);
  render(<RouterProvider router={router} />);
  await act(() => router.load());

  return router;
}

afterEach(() => vi.restoreAllMocks());

describe("MessageResolver", () => {
  it("shows the shared loading state, then replaces the bare link with its permalink", async () => {
    const pending = Promise.withResolvers<MessageRead>();
    const read = vi.spyOn(actions.messages, "read").mockReturnValue(pending.promise);
    const router = await mount("/app/m/23?source=classic&source=link#top");

    expect(document.querySelector('[aria-busy="true"]')).toBeTruthy();
    expect(read).toHaveBeenCalledWith(23);
    expect(router.history.location.state.smartfireMessageLink).toEqual({
      messageId: 23,
      search: "?source=classic&source=link",
      hash: "#top",
    });
    expect(router.state.location.search).toEqual({ source: ["classic", "link"] });
    await act(async () => pending.resolve(messageRead(23)));
    await waitFor(() =>
      expect(router.history.location.href).toBe("/app/r/4/m/23?source=classic&source=link#top"),
    );
    expect(router.history.location.state.smartfireMessageLink).toBeUndefined();
    expect(read).toHaveBeenCalledTimes(1);
    await act(async () => router.history.back());
    await screen.findByText("Home destination");
    expect(router.state.location.pathname).toBe("/");
  });

  it("resolves a reply to its existing thread with that reply focused", async () => {
    vi.spyOn(actions.messages, "read").mockResolvedValue(messageRead(23, 7));
    const router = await mount("/app/m/23?m=9&source=classic&source=link#top");

    await screen.findByText("Thread destination");
    expect(router.history.location.href).toBe("/app/r/4/t/7?m=23&source=classic&source=link#top");
  });

  it.each([
    ["1e3", 1000],
    ["0x17", 23],
    ["%31", 1],
  ])(
    "resolves the route's accepted id spelling %s instead of waiting forever",
    async (id, value) => {
      const read = vi.spyOn(actions.messages, "read").mockResolvedValue(messageRead(value));
      const router = await mount(`/app/m/${id}`);

      await screen.findByText("Room destination");
      expect(read).toHaveBeenCalledWith(value);
      expect(router.history.location.href).toBe(`/app/r/4/m/${value}`);
    },
  );

  it.each(["Forbidden", "NotFound"])("keeps an API %s on the unavailable page", async (tag) => {
    vi.spyOn(actions.messages, "read").mockRejectedValue(new ActionError(tag, "Unavailable"));
    const router = await mount("/app/m/23");

    await screen.findByRole("region", { name: "Page not found" });
    expect(router.history.location.href).toBe("/app/m/23");
    expect(screen.queryByText("Room destination")).toBeNull();
  });

  it("ignores an older message response after another bare message is opened", async () => {
    const older = Promise.withResolvers<MessageRead>();
    const newer = Promise.withResolvers<MessageRead>();

    const read = vi
      .spyOn(actions.messages, "read")
      .mockReturnValueOnce(older.promise)
      .mockReturnValueOnce(newer.promise);

    const router = await mount("/app/m/23");

    await act(() => router.navigate({ href: "/app/m/24" }));
    await waitFor(() => expect(read).toHaveBeenCalledWith(24));
    await act(async () => older.resolve(messageRead(23)));
    expect(router.history.location.href).toBe("/app/m/24");
    await act(async () => newer.resolve(messageRead(24)));
    await screen.findByText("Room destination");
    expect(router.history.location.href).toBe("/app/r/4/m/24");
  });

  it("ignores a late failure after the resolver has been left", async () => {
    const pending = Promise.withResolvers<MessageRead>();

    vi.spyOn(actions.messages, "read").mockReturnValue(pending.promise);
    const router = await mount("/app/m/23");

    await act(() => router.navigate({ href: "/app/" }));
    await act(async () => pending.reject(new ActionError("NotFound", "Unavailable")));
    expect(screen.queryByRole("region", { name: "Page not found" })).toBeNull();
    expect(screen.getByText("Home destination")).toBeTruthy();
  });
});
