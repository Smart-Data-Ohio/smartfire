import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { SEED_IDS } from "../../../mock/server.ts";
import type { CreateMessage } from "../../gen/CreateMessage.ts";
import type { Me } from "../../gen/Me.ts";
import type { MessagePage } from "../../gen/MessagePage.ts";
import type { MessageDTO } from "../../store/model.ts";
import { mutations, store } from "../../store/store.ts";
import { installMockNetwork, type MockNetwork } from "../../test/mock-network.ts";
import { Composer } from "./composer.tsx";
import { draftKey } from "./draft.ts";
import {
  cancelReply,
  replyKey,
  replyTarget,
  resetReplies,
  setReplyNotify,
  startReply,
} from "./reply-store.ts";

// The composer sends through the real outbox against the in-memory mock backend.
let network: MockNetwork;

const ROOM = SEED_IDS.rooms.general;

const VIEWER = SEED_IDS.viewer;

/** Every message create the composer posted, parsed. */
let posts: CreateMessage[] = [];

/** A loaded #general message by someone other than the viewer. */
function seeded(): MessageDTO {
  const found = Object.values(store.getState().messages).find(
    (message) => message.roomId === ROOM && message.creatorId !== VIEWER && !message.systemNote,
  );

  if (found === undefined) {
    throw new Error("the seed has no such message");
  }

  return found;
}

beforeAll(async () => {
  network = installMockNetwork();

  const meta = document.createElement("meta");

  meta.name = "csrf-token";
  meta.content = network.server.csrfToken();
  document.head.append(meta);

  const mocked = globalThis.fetch;

  globalThis.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = input instanceof Request ? input : new URL(String(input), location.href);
    const request = new Request(url, init);

    if (request.method === "POST" && new URL(request.url).pathname.endsWith("/messages")) {
      posts.push(await request.clone().json());
    }

    return mocked(input, init);
  };
});

afterAll(() => network.restore());

beforeEach(async () => {
  // jsdom has no matchMedia; the motion helpers ask it about reduced motion.
  window.matchMedia = (query: string) =>
    Object.assign(new EventTarget(), {
      matches: false,
      media: query,
      onchange: null,
      addListener: () => undefined,
      removeListener: () => undefined,
    });

  // jsdom has no ResizeObserver; the send button's goo effect watches its size.
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

  mutations.reset();
  mutations.setMe(me);
  mutations.applyPage(ROOM, page, "replace");
  resetReplies();
  sessionStorage.clear();
  posts = [];
});

describe("reply targets", () => {
  it("start notifying the author, per conversation, until cancelled", () => {
    const message = seeded();
    const room = draftKey(ROOM, null);

    startReply(message);

    expect(replyKey(message)).toBe(room);
    expect(replyTarget(room)).toMatchObject({ messageId: message.id, notify: true });
    expect(replyTarget(draftKey(ROOM, 88))).toBeNull();

    setReplyNotify(room, false);
    expect(replyTarget(room)?.notify).toBe(false);

    // Picking again starts over: notify is back on.
    startReply(message);
    expect(replyTarget(room)?.notify).toBe(true);

    cancelReply(room);
    expect(replyTarget(room)).toBeNull();
  });

  it("go to the thread's composer for a reply inside a thread", () => {
    startReply({ id: 5, roomId: ROOM, threadId: 88 });

    expect(replyTarget(draftKey(ROOM, 88))?.messageId).toBe(5);
    expect(replyTarget(draftKey(ROOM, null))).toBeNull();
  });
});

describe("the composer's reply", () => {
  const input = () => screen.getByRole("textbox");

  const chip = () => screen.queryByRole("region", { name: /^Replying to / });

  it("shows the quote with Notify author on, and takes the focus", async () => {
    const message = seeded();

    render(<Composer roomId={ROOM} />);
    act(() => startReply(message));

    expect(chip()).not.toBeNull();
    expect(chip()?.textContent).toContain("Replying to");
    expect(screen.getByRole("checkbox", { name: "Notify author" })).toHaveProperty("checked", true);
    await waitFor(() => expect(document.activeElement).toBe(input()));
  });

  it("hides Notify author on a reply to yourself", async () => {
    const response = await fetch(`/api/v1/rooms/${ROOM}/messages`, {
      method: "POST",
      headers: { "Content-Type": "application/json", "X-CSRF-Token": network.server.csrfToken() },
      body: JSON.stringify({
        clientMessageId: "reply-own",
        markdownSource: "Mine",
        replyToMessageId: null,
        replyNotifyAuthor: null,
      }),
    });

    const own: MessageDTO = await response.json();

    act(() => mutations.receiveMessage(own));
    render(<Composer roomId={ROOM} />);
    act(() => startReply(own));

    expect(chip()).not.toBeNull();
    expect(screen.queryByRole("checkbox", { name: "Notify author" })).toBeNull();
  });

  it("cancels with × and with Escape, keeping the draft", async () => {
    const user = userEvent.setup();

    render(<Composer roomId={ROOM} />);
    act(() => startReply(seeded()));
    await user.click(screen.getByRole("button", { name: "Cancel reply" }));

    expect(chip()).toBeNull();

    act(() => startReply(seeded()));
    await user.type(input(), "draft");
    await user.keyboard("{Escape}");

    expect(chip()).toBeNull();
    expect(input()).toHaveProperty("value", "draft");
  });

  it("sends the reply fields, then drops the reply", async () => {
    const user = userEvent.setup();
    const message = seeded();

    render(<Composer roomId={ROOM} />);
    act(() => startReply(message));
    await user.type(input(), "On it{Enter}");

    await waitFor(() => expect(posts).toHaveLength(1));
    expect(posts[0]).toMatchObject({
      markdownSource: "On it",
      replyToMessageId: message.id,
      replyNotifyAuthor: true,
    });
    expect(chip()).toBeNull();

    await waitFor(() => {
      const sent = Object.values(store.getState().messages).find(
        (candidate) => candidate.clientMessageId === posts[0]?.clientMessageId,
      );

      expect(sent?.replyToMessageId).toBe(message.id);
    });
  });

  it("sends Notify author off when it's unticked", async () => {
    const user = userEvent.setup();
    const message = seeded();

    render(<Composer roomId={ROOM} />);
    act(() => startReply(message));
    await user.click(screen.getByRole("checkbox", { name: "Notify author" }));
    await user.type(input(), "Quietly{Enter}");

    await waitFor(() => expect(posts).toHaveLength(1));
    expect(posts[0]).toMatchObject({ replyToMessageId: message.id, replyNotifyAuthor: false });
  });

  it("sends a plain message with no reply fields set", async () => {
    const user = userEvent.setup();

    render(<Composer roomId={ROOM} />);
    await user.type(input(), "Hello{Enter}");

    await waitFor(() => expect(posts).toHaveLength(1));
    expect(posts[0]).toMatchObject({ replyToMessageId: null, replyNotifyAuthor: null });
  });
});
