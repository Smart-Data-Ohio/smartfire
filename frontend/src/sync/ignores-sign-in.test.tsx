import {
  createMemoryHistory,
  createRootRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, screen, waitFor } from "@testing-library/react";
import { afterAll, afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { meFixture, userFixture } from "../api/testing.ts";
import { BlockedSection } from "../features/settings/blocked-section.tsx";
import type { BlockList } from "../gen/BlockList.ts";
import type { BlockState } from "../gen/BlockState.ts";
import { mutations, store } from "../store/store.ts";
import { ignores } from "./admin.ts";
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
