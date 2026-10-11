import {
  createMemoryHistory,
  createRootRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterAll, afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { meFixture, userFixture } from "../api/testing.ts";
import { BlockedSection } from "../features/settings/blocked-section.tsx";
import { AccountChangedNotice } from "../features/shell/account-changed.tsx";
import type { BlockList } from "../gen/BlockList.ts";
import type { BlockState } from "../gen/BlockState.ts";
import { mutations, store } from "../store/store.ts";
import { ignores } from "./admin.ts";
import { actions } from "./runtime.ts";
import { pageReload, stopRunForNewSignIn } from "./sign-in-change.ts";

const A = meFixture.user.id;

const B = A + 1;

const X = 5;

const ONLY_B = 6;

const ignoredSince = "2026-10-01T12:00:00Z";

/** A ignores X: the list this page holds. */
const listOfA: BlockList = {
  viewerId: A,
  revision: 3,
  people: [{ user: userFixture(X, "Xavier"), ignoredByMe: true, createdAt: ignoredSince }],
};

/** B ignores X too, and someone A doesn't. */
const listOfB: BlockList = {
  viewerId: B,
  revision: 9,
  people: [
    { user: userFixture(X, "Xavier"), ignoredByMe: true, createdAt: ignoredSince },
    { user: userFixture(ONLY_B, "Bea's pick"), ignoredByMe: true, createdAt: ignoredSince },
  ],
};

/** What the server answers the ignore request with. */
let answer: BlockList | BlockState = listOfA;

// The action runtime keeps the `fetch` it first sees, so the stub stays for the whole file.
vi.spyOn(globalThis, "fetch").mockImplementation((input) => {
  // A write first reads the CSRF token.
  const reply = new URL(String(input), "http://campfire.test").pathname.endsWith("/boot")
    ? { csrfToken: "token" }
    : answer;

  return Promise.resolve(new Response(JSON.stringify(reply)));
});

const reload = vi.spyOn(pageReload, "reload").mockImplementation(() => undefined);

/** Whether `promise` is still pending once everything queued so far has run. */
async function pending(promise: Promise<unknown>): Promise<boolean> {
  const still = Symbol("pending");

  const first = await Promise.race([
    promise,
    new Promise((resolve) => setTimeout(() => resolve(still), 20)),
  ]);

  return first === still;
}

beforeEach(() => {
  reload.mockClear();
  mutations.reset();
  mutations.setMe(meFixture);
  mutations.setBlocks(listOfA);
});

afterEach(() => mutations.reset());

afterAll(() => vi.restoreAllMocks());

describe("ignore replies answered for another sign-in", () => {
  it("never hand B's list to the caller or the store, and leave the page", async () => {
    const before = store.getState().ignores;

    answer = listOfB;

    const list = ignores.list();

    expect(await pending(list)).toBe(true);
    expect(reload).toHaveBeenCalledTimes(1);
    expect(store.getState().ignores).toEqual(before);
    expect(store.getState().users[ONLY_B]).toBeUndefined();
  });

  it("never apply B's ignore write reply", async () => {
    const before = store.getState().ignores;

    answer = { viewerId: B, userId: X, ignoredByMe: false, revision: 10 };

    const change = ignores.set(X, false);

    expect(await pending(change)).toBe(true);
    expect(reload).toHaveBeenCalledTimes(1);
    expect(store.getState().ignores).toEqual(before);
  });

  it("stop a running sync engine first, which reloads once it has stopped", async () => {
    const stop = vi.fn();
    const release = stopRunForNewSignIn(stop);

    answer = listOfB;

    try {
      expect(await pending(ignores.list())).toBe(true);
    } finally {
      release();
    }

    expect(stop).toHaveBeenCalledTimes(1);
    expect(reload).not.toHaveBeenCalled();
  });

  it("land a reply answered for this page as before", async () => {
    answer = { ...listOfA, revision: 4 };

    expect((await ignores.list()).people.map((person) => person.user.id)).toEqual([X]);
    expect(store.getState().ignores.snapshot).toBe(4);
    expect(reload).not.toHaveBeenCalled();
  });
});

describe("Ignored people under another sign-in", () => {
  it("shows nothing of B's when it opens before the engine sees the new sign-in", async () => {
    // A ignores X; the cookie now signs in B, who ignores X too, and the page hasn't noticed.
    const before = store.getState().ignores;

    answer = listOfB;

    const root = createRootRoute({ component: BlockedSection });

    const router = createRouter({
      routeTree: root,
      history: createMemoryHistory({ initialEntries: ["/"] }),
    });

    render(<RouterProvider router={router} />);
    await act(() => router.load());

    await waitFor(() => expect(reload).toHaveBeenCalledTimes(1));
    // B's reply never settles the load: no list of theirs, no empty state, no error.
    expect(screen.queryByRole("link")).toBeNull();
    expect(screen.queryByText("You're not ignoring anyone")).toBeNull();
    expect(screen.queryByRole("button", { name: /try again|retry/i })).toBeNull();
    expect(store.getState().ignores).toEqual(before);
    expect(store.getState().users[ONLY_B]).toBeUndefined();
  });
});

describe("once another account answered, with the reload held back", () => {
  /** B's write reply lands on A's page; the browser doesn't carry out the reload. */
  async function answeredAsB() {
    answer = { viewerId: B, userId: X, ignoredByMe: false, revision: 10 };
    await pending(ignores.set(X, false));
    expect(reload).toHaveBeenCalledTimes(1);
  }

  it("sends no more requests, and the connection reads offline", async () => {
    await answeredAsB();

    const sent = vi.mocked(globalThis.fetch).mock.calls.length;

    answer = listOfA;
    await expect(ignores.list()).rejects.toThrow("different account");
    await expect(ignores.set(X, true)).rejects.toThrow("different account");

    expect(vi.mocked(globalThis.fetch).mock.calls.length).toBe(sent);
    expect(store.getState()).toMatchObject({ accountChanged: true, connection: "offline" });

    // Nothing brings it back online but a reload.
    mutations.setConnection("online");
    expect(store.getState().connection).toBe("offline");
  });

  it("installs nothing when a room is opened", async () => {
    await answeredAsB();

    const frozen = store.getState();
    const sent = vi.mocked(globalThis.fetch).mock.calls.length;

    await pending(actions.openRoom(12, null).catch(() => undefined));

    expect(vi.mocked(globalThis.fetch).mock.calls.length).toBe(sent);
    expect(store.getState()).toBe(frozen);
  });

  it("puts up the notice, whose Reload reloads", () => {
    render(<AccountChangedNotice />);

    expect(
      screen.getByRole("heading", {
        name: "You signed in as a different account in another tab",
      }),
    ).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Reload" }));
    expect(reload).toHaveBeenCalledTimes(1);
  });
});
