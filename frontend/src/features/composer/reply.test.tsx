import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { SEED_IDS } from "../../../mock/server.ts";
import type { CreateMessage } from "../../gen/CreateMessage.ts";
import type { CreateScheduledMessage } from "../../gen/CreateScheduledMessage.ts";
import type { Me } from "../../gen/Me.ts";
import type { MessagePage } from "../../gen/MessagePage.ts";
import { uuid7 } from "../../lib/uuid7.ts";
import type { MessageDTO } from "../../store/model.ts";
import { mutations, store } from "../../store/store.ts";
import { composerActions } from "../../sync/composer-actions.ts";
import { actions } from "../../sync/runtime.ts";
import { installMockNetwork, type MockNetwork } from "../../test/mock-network.ts";
import { editingId, stopEditing } from "../messages/editing-store.ts";
import { Composer } from "./composer.tsx";
import { draftKey } from "./draft.ts";
import {
  cancelReply,
  replyKey,
  replyTarget,
  resetReplies,
  setReplyNotify,
  startReply,
  trackSentReply,
} from "./reply-store.ts";

// The composer sends through the real outbox against the in-memory mock backend.
let network: MockNetwork;

const ROOM = SEED_IDS.rooms.general;

const VIEWER = SEED_IDS.viewer;

/** Every message create the composer posted, parsed. */
let posts: CreateMessage[] = [];

/** Every scheduled-message create, parsed. */
let schedules: CreateScheduledMessage[] = [];

/** Which message creates the backend refuses with a 422; `null` takes them all. */
let refuse: ((body: CreateMessage) => boolean) | null = null;

/** What a refusal waits on before it answers, to let the test act while the send is in flight. */
let hold: Promise<void> = Promise.resolve();

/** POSTs to paths ending in `suffix` wait for `until` before they go through. */
let delay: { readonly suffix: string; readonly until: Promise<void> } | null = null;

/** A loaded #general message by someone other than the viewer (not `except`). */
function seeded(except: number | null = null): MessageDTO {
  const found = Object.values(store.getState().messages).find(
    (message) =>
      message.roomId === ROOM &&
      message.creatorId !== VIEWER &&
      !message.systemNote &&
      message.id !== except,
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

  // jsdom doesn't lay out; the command list scrolls its active option into view.
  Element.prototype.scrollIntoView = () => undefined;

  const mocked = globalThis.fetch;

  globalThis.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = input instanceof Request ? input : new URL(String(input), location.href);
    const request = new Request(url, init);

    const path = new URL(request.url).pathname;

    if (request.method === "POST" && path.endsWith("/scheduled_messages")) {
      schedules.push(await request.clone().json());
    } else if (request.method === "POST" && path.endsWith("/messages")) {
      const body: CreateMessage = await request.clone().json();

      posts.push(body);

      if (refuse?.(body)) {
        await hold;

        return Response.json({ error: "Not today" }, { status: 422 });
      }
    }

    if (request.method === "POST" && delay !== null && path.endsWith(delay.suffix)) {
      await delay.until;
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
  stopEditing();
  posts = [];
  schedules = [];
  refuse = null;
  hold = Promise.resolve();
  delay = null;
});

/** The pending messages that failed for good. */
function failedSends() {
  return Object.values(store.getState().pending).filter((pending) => pending.state === "failed");
}

/** Posts a message as the viewer and loads it, as their own last message. */
async function postOwn(): Promise<MessageDTO> {
  const response = await fetch(`/api/v1/rooms/${ROOM}/messages`, {
    method: "POST",
    headers: { "Content-Type": "application/json", "X-CSRF-Token": network.server.csrfToken() },
    body: JSON.stringify({
      clientMessageId: uuid7(Date.now()),
      markdownSource: "Mine",
      replyToMessageId: null,
      replyNotifyAuthor: null,
    }),
  });

  const own: MessageDTO = await response.json();

  act(() => mutations.receiveMessage(own));
  posts = [];

  return own;
}

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
    const own = await postOwn();

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

  it("comes back with its notify choice when the text send fails", async () => {
    const user = userEvent.setup();
    const message = seeded();

    refuse = () => true;
    render(<Composer roomId={ROOM} />);
    act(() => startReply(message));
    await user.click(screen.getByRole("checkbox", { name: "Notify author" }));
    await user.type(input(), "Lost{Enter}");

    await waitFor(() => expect(failedSends()).toHaveLength(1));
    await waitFor(() => expect(chip()).not.toBeNull());
    expect(replyTarget(draftKey(ROOM, null))).toMatchObject({
      messageId: message.id,
      notify: false,
    });
    expect(screen.getByRole("checkbox", { name: "Notify author" })).toHaveProperty(
      "checked",
      false,
    );
    expect(failedSends()[0]).toMatchObject({ replyToMessageId: message.id });
  });

  // jsdom can't run the direct upload, so this sends what the composer sends for text plus one
  // file (the mock e2e attaches a real file through the composer).
  it("rides on each file message, and comes back when a file message fails", async () => {
    const message = seeded();
    const key = draftKey(ROOM, null);
    const textId = uuid7(Date.now());
    const fileId = uuid7(Date.now());
    const ids = [textId, fileId];

    refuse = (body) => body.attachmentSignedId !== null;
    render(<Composer roomId={ROOM} />);
    act(() => startReply(message));

    const target = replyTarget(key);

    expect(target).not.toBeNull();

    const reply = { messageId: message.id, notify: true };

    act(() => {
      actions.send(ROOM, "Notes attached", { reply, clientMessageId: textId });
      actions.send(ROOM, "", {
        attachmentSignedId: "signed-notes",
        reply,
        clientMessageId: fileId,
      });
      cancelReply(key);

      if (target !== null) trackSentReply(key, target, ids);
    });

    await waitFor(() =>
      expect(failedSends().map((pending) => pending.clientMessageId)).toEqual([fileId]),
    );
    await waitFor(() => expect(chip()).not.toBeNull());
    expect(new Set(posts.map((post) => post.clientMessageId))).toEqual(new Set(ids));
    expect(posts.every((post) => post.replyToMessageId === message.id)).toBe(true);
  });

  it("doesn't come back over a reply picked while the failed send was in flight", async () => {
    const user = userEvent.setup();
    const first = seeded();
    const second = seeded(first.id);
    const gate = Promise.withResolvers<void>();

    hold = gate.promise;
    refuse = () => true;
    render(<Composer roomId={ROOM} />);
    act(() => startReply(first));
    await user.type(input(), "Slow{Enter}");
    await waitFor(() => expect(posts).toHaveLength(1));
    expect(chip()).toBeNull();

    act(() => startReply(second));
    gate.resolve();

    await waitFor(() => expect(failedSends()).toHaveLength(1));
    expect(replyTarget(draftKey(ROOM, null))?.messageId).toBe(second.id);
  });

  it("is resent by Retry, and leaves the composer once that lands", async () => {
    const user = userEvent.setup();
    const message = seeded();

    refuse = () => true;
    render(<Composer roomId={ROOM} />);
    act(() => startReply(message));
    await user.type(input(), "Again{Enter}");
    await waitFor(() => expect(chip()).not.toBeNull());

    const [failed] = failedSends();

    refuse = null;
    act(() => actions.retry(failed?.clientMessageId ?? ""));

    await waitFor(() => expect(posts).toHaveLength(2));
    expect(posts[1]).toMatchObject({
      clientMessageId: failed?.clientMessageId,
      replyToMessageId: message.id,
      replyNotifyAuthor: true,
    });
    await waitFor(() => expect(chip()).toBeNull());
  });

  it("stays with its room when the composer switches rooms, without taking the focus back", async () => {
    const message = seeded();
    const view = render(<Composer roomId={ROOM} />);

    act(() => startReply(message));
    await waitFor(() => expect(document.activeElement).toBe(input()));
    act(() => input().blur());

    view.rerender(<Composer roomId={SEED_IDS.rooms.design} />);
    expect(chip()).toBeNull();

    view.rerender(<Composer roomId={ROOM} />);
    expect(chip()).not.toBeNull();
    expect(document.activeElement).not.toBe(input());
  });

  it("survives ↑ editing your last message", async () => {
    const user = userEvent.setup();
    const own = await postOwn();
    const message = seeded();

    render(<Composer roomId={ROOM} />);
    act(() => startReply(message));
    await waitFor(() => expect(document.activeElement).toBe(input()));
    await user.keyboard("{ArrowUp}");

    expect(editingId()).toBe(own.id);
    expect(replyTarget(draftKey(ROOM, null))?.messageId).toBe(message.id);
    expect(chip()).not.toBeNull();
  });

  it("goes with a scheduled send and leaves once it's scheduled", async () => {
    const user = userEvent.setup();
    const message = seeded();

    render(<Composer roomId={ROOM} />);
    act(() => startReply(message));
    await user.type(input(), "Later");
    await user.click(screen.getByRole("button", { name: "Schedule message" }));

    const [preset] = within(
      await screen.findByRole("menu", { name: "Schedule message" }),
    ).getAllByRole("menuitem");

    expect(preset).toBeDefined();

    if (preset !== undefined) await user.click(preset);

    await waitFor(() => expect(schedules).toHaveLength(1));
    expect(schedules[0]).toMatchObject({ markdownSource: "Later", replyToMessageId: message.id });
    await waitFor(() => expect(chip()).toBeNull());
    expect(posts).toHaveLength(0);
  });

  it("isn't started by typing Q in the composer", async () => {
    const user = userEvent.setup();

    render(<Composer roomId={ROOM} />);
    await user.type(input(), "q");
    await user.keyboard("{Shift>}Q{/Shift}");

    expect(chip()).toBeNull();
    expect(replyTarget(draftKey(ROOM, null))).toBeNull();
    expect(input()).toHaveProperty("value", "qQ");
  });

  it("doesn't come back after a reply picked and cancelled while the failed send was out", async () => {
    const user = userEvent.setup();
    const first = seeded();
    const second = seeded(first.id);
    const gate = Promise.withResolvers<void>();
    const key = draftKey(ROOM, null);

    hold = gate.promise;
    refuse = () => true;
    render(<Composer roomId={ROOM} />);
    act(() => startReply(first));
    await user.type(input(), "Slow{Enter}");
    await waitFor(() => expect(posts).toHaveLength(1));

    act(() => startReply(second));
    act(() => cancelReply(key));
    gate.resolve();

    await waitFor(() => expect(failedSends()).toHaveLength(1));
    expect(replyTarget(key)).toBeNull();
    expect(chip()).toBeNull();
  });

  it("keeps a reply picked while a schedule request was out", async () => {
    const user = userEvent.setup();
    const first = seeded();
    const second = seeded(first.id);
    const gate = Promise.withResolvers<void>();

    delay = { suffix: "/scheduled_messages", until: gate.promise };
    render(<Composer roomId={ROOM} />);
    act(() => startReply(first));
    await user.type(input(), "Later");
    await user.click(screen.getByRole("button", { name: "Schedule message" }));

    const [preset] = within(
      await screen.findByRole("menu", { name: "Schedule message" }),
    ).getAllByRole("menuitem");

    if (preset !== undefined) await user.click(preset);

    await waitFor(() => expect(schedules).toHaveLength(1));
    expect(schedules[0]).toMatchObject({ replyToMessageId: first.id });

    act(() => startReply(second));
    gate.resolve();

    await waitFor(() => expect(input()).toHaveProperty("value", ""));
    expect(replyTarget(draftKey(ROOM, null))?.messageId).toBe(second.id);
  });

  it("keeps a reply picked while a slash command was running", async () => {
    const user = userEvent.setup();
    const first = seeded();
    const second = seeded(first.id);
    const gate = Promise.withResolvers<void>();
    const run = vi.spyOn(composerActions, "runSlashCommand");

    delay = { suffix: "/slash_commands", until: gate.promise };
    render(<Composer roomId={ROOM} />);
    act(() => startReply(first));
    await user.type(input(), "/shrug fine{Enter}");
    await waitFor(() => expect(run).toHaveBeenCalledTimes(1));

    act(() => startReply(second));
    gate.resolve();
    await act(async () => {
      await expect(run.mock.results[0]?.value).resolves.toMatchObject({ status: "posted" });
    });

    expect(replyTarget(draftKey(ROOM, null))?.messageId).toBe(second.id);
    run.mockRestore();
  });

  it("is taken by a slash command that runs", async () => {
    const user = userEvent.setup();
    const run = vi.spyOn(composerActions, "runSlashCommand");

    render(<Composer roomId={ROOM} />);
    act(() => startReply(seeded()));
    await user.type(input(), "/shrug fine{Enter}");
    await waitFor(() => expect(run).toHaveBeenCalledTimes(1));
    await act(async () => {
      await expect(run.mock.results[0]?.value).resolves.toMatchObject({ status: "posted" });
    });

    expect(replyTarget(draftKey(ROOM, null))).toBeNull();
    run.mockRestore();
  });
});
