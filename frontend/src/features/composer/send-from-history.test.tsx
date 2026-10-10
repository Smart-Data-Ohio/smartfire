import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { SEED_IDS } from "../../../mock/server.ts";
import type { CreateMessage } from "../../gen/CreateMessage.ts";
import type { Me } from "../../gen/Me.ts";
import type { MessagePage } from "../../gen/MessagePage.ts";
import { mutations, store } from "../../store/store.ts";
import { installMockNetwork, type MockNetwork } from "../../test/mock-network.ts";
import { clearCommandCache } from "./autocomplete/suggestions.ts";
import { Composer } from "./composer.tsx";
import { resetReplies } from "./reply-store.ts";

// The composer sends through the real outbox and jumps through the real session, against the
// in-memory mock backend, with the latest-page GET held open.
let network: MockNetwork;

const ROOM = SEED_IDS.rooms.general;

/** Every message create the composer posted, parsed. */
let posts: CreateMessage[] = [];

/** What the room's latest-page GET waits on. */
let latestPage: Promise<void> = Promise.resolve();

/** How many latest-page GETs went out. */
let latestPageGets = 0;

beforeAll(() => {
  network = installMockNetwork();

  const meta = document.createElement("meta");

  meta.name = "csrf-token";
  meta.content = network.server.csrfToken();
  document.head.append(meta);

  const mocked = globalThis.fetch;

  globalThis.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = input instanceof Request ? input : new URL(String(input), location.href);
    const request = new Request(url, init);
    const { pathname, search } = new URL(request.url);
    const messages = pathname === `/api/v1/rooms/${ROOM}/messages`;

    if (messages && request.method === "POST") {
      posts.push(await request.clone().json());
    } else if (messages && request.method === "GET" && search === "") {
      latestPageGets += 1;
      await latestPage;
    }

    return mocked(input, init);
  };
});

afterAll(() => network.restore());

beforeEach(async () => {
  window.matchMedia = (query: string) =>
    Object.assign(new EventTarget(), {
      matches: false,
      media: query,
      onchange: null,
      addListener: () => undefined,
      removeListener: () => undefined,
    });

  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  );

  const me: Me = await (await fetch("/api/v1/me")).json();
  const page: MessagePage = await (await fetch(`/api/v1/rooms/${ROOM}/messages`)).json();
  const newest = page.messages.at(-1);

  if (newest === undefined) {
    throw new Error("the seed has no messages in #general");
  }

  mutations.reset();
  mutations.setMe(me);
  // Scrolled back into history: there's a newer page past this window.
  mutations.applyPage(ROOM, { ...page, after: newest.id }, "replace");
  resetReplies();
  sessionStorage.clear();
  clearCommandCache();
  posts = [];
  latestPage = Promise.resolve();
  latestPageGets = 0;
});

describe("an ordinary send from history", () => {
  it("posts, shows its row and frees Send before the latest page arrives", async () => {
    const gate = Promise.withResolvers<void>();

    latestPage = gate.promise;
    expect(store.getState().timelines[ROOM]?.after).not.toBeNull();

    render(<Composer roomId={ROOM} placeholder="Message" />);
    const input = screen.getByRole("textbox", { name: "Message" });
    const sendButton = () => screen.getByRole("button", { name: "Send message" });

    const pendingTexts = () =>
      Object.values(store.getState().pending).map((pending) => pending.markdownSource);

    fireEvent.change(input, { target: { value: "First from history" } });
    fireEvent.keyDown(input, { key: "Enter" });

    expect(pendingTexts()).toEqual(["First from history"]);
    await waitFor(() =>
      expect(posts.map((post) => post.markdownSource)).toEqual(["First from history"]),
    );
    expect(latestPageGets).toBe(1);

    fireEvent.change(input, { target: { value: "Second from history" } });
    expect(sendButton()).toHaveProperty("disabled", false);
    fireEvent.click(sendButton());

    expect(pendingTexts()).toContain("Second from history");
    await waitFor(() =>
      expect(posts.map((post) => post.markdownSource)).toEqual([
        "First from history",
        "Second from history",
      ]),
    );
    expect(latestPageGets).toBeGreaterThanOrEqual(1);

    await act(async () => gate.resolve());
    await waitFor(() => expect(store.getState().timelines[ROOM]?.after).toBeNull());
  });
});
