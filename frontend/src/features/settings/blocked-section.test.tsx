import {
  createMemoryHistory,
  createRootRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { userFixture } from "../../api/testing.ts";
import type { BlockList } from "../../gen/BlockList.ts";
import { mutations, store } from "../../store/store.ts";
import { ignores } from "../../sync/admin.ts";
import { BlockedSection } from "./blocked-section.tsx";

afterEach(() => {
  vi.restoreAllMocks();
  mutations.reset();
});

const LIST: BlockList = {
  revision: 4,
  people: [
    { user: userFixture(2, "Maya"), ignoredByMe: true, createdAt: "2026-10-01T12:00:00Z" },
    { user: userFixture(3, "Jason"), ignoredByMe: true, createdAt: "2026-10-02T12:00:00Z" },
  ],
};

/** Mounts the section, its list landing in the store as the facade's does. */
async function mount(list: BlockList) {
  vi.spyOn(ignores, "list").mockImplementation(() => {
    mutations.setBlocks(list);

    return Promise.resolve(list);
  });

  const root = createRootRoute({ component: BlockedSection });

  const router = createRouter({
    routeTree: root,
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());
}

it("lists the people you ignore, each linking to their page", async () => {
  await mount(LIST);

  expect(await screen.findByRole("link", { name: "Maya" })).toBeTruthy();
  expect(screen.getByRole("link", { name: "Jason" }).getAttribute("href")).toBe("/people/3");
  expect(screen.getByRole("button", { name: "Stop ignoring Maya" })).toBeTruthy();
  expect(screen.getByText("Only you see this list.", { exact: false })).toBeTruthy();
});

it("says so when you ignore nobody", async () => {
  await mount({ revision: 0, people: [] });

  expect(await screen.findByText("You're not ignoring anyone")).toBeTruthy();
  expect(screen.queryByRole("list")).toBeNull();
});

it("stops ignoring someone and drops them from the list", async () => {
  await mount(LIST);

  const set = vi.spyOn(ignores, "set").mockImplementation((userId, ignored) => {
    const change = { userId, ignoredByMe: ignored, revision: 5 };

    mutations.applyBlockState(change);

    return Promise.resolve(change);
  });

  fireEvent.click(await screen.findByRole("button", { name: "Stop ignoring Maya" }));

  expect(set).toHaveBeenCalledWith(2, false);
  await waitFor(() => expect(screen.queryByRole("link", { name: "Maya" })).toBeNull());
  expect(screen.getByRole("link", { name: "Jason" })).toBeTruthy();
  expect(store.getState().ignores.people[2]).toBe(false);
});

it("keeps a stopped person gone when the write's sync echo refetches the list", async () => {
  await mount(LIST);

  vi.spyOn(ignores, "set").mockImplementation((userId, ignored) => {
    const change = { userId, ignoredByMe: ignored, revision: 5 };

    mutations.applyBlockState(change);

    return Promise.resolve(change);
  });

  fireEvent.click(await screen.findByRole("button", { name: "Stop ignoring Maya" }));
  await waitFor(() => expect(screen.queryByRole("link", { name: "Maya" })).toBeNull());

  // The echo's `GET /blocks` at the write's revision: Maya isn't on it, so nothing says "ignored".
  act(() => mutations.setBlocks({ revision: 5, people: LIST.people.slice(1) }));

  expect(screen.queryByRole("link", { name: "Maya" })).toBeNull();
  expect(screen.getByRole("link", { name: "Jason" })).toBeTruthy();

  // A snapshot older than the write, landing late, doesn't bring her back either.
  act(() => mutations.setBlocks(LIST));
  expect(screen.queryByRole("link", { name: "Maya" })).toBeNull();
});

it("drops someone you stopped ignoring elsewhere as the change lands", async () => {
  await mount(LIST);
  await screen.findByRole("link", { name: "Jason" });

  act(() => mutations.applyBlockState({ userId: 3, ignoredByMe: false, revision: 9 }));

  expect(screen.queryByRole("link", { name: "Jason" })).toBeNull();
  expect(screen.getByRole("link", { name: "Maya" })).toBeTruthy();
});

it("offers a retry when the list doesn't load", async () => {
  vi.spyOn(ignores, "list").mockRejectedValueOnce(new Error("Offline"));

  const root = createRootRoute({ component: BlockedSection });

  const router = createRouter({
    routeTree: root,
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());

  expect(await screen.findByText("Offline")).toBeTruthy();
  vi.spyOn(ignores, "list").mockResolvedValue(LIST);
  fireEvent.click(screen.getByRole("button", { name: /try again|retry/i }));
  expect(await screen.findByRole("link", { name: "Maya" })).toBeTruthy();
});
