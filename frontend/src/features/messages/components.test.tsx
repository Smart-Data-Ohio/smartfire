import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterAll, beforeAll, beforeEach, describe, expect, it } from "vitest";
import { SEED_IDS } from "../../../mock/server.ts";
import { messageFixture, pageFixture, userFixture } from "../../api/testing.ts";
import type { Me } from "../../gen/Me.ts";
import { resetRecentEmoji } from "../../lib/emoji/recent.ts";
import type { MessageDTO } from "../../store/model.ts";
import { mutations, store } from "../../store/store.ts";
import { installMockNetwork, type MockNetwork } from "../../test/mock-network.ts";
import { editLastOwnMessage, lastEditableMessage } from "./edit-last.ts";
import { editingId, stopEditing } from "./editing-store.ts";
import { editorKeyAction, MessageEditor } from "./message-editor.tsx";
import { ReactionsRow } from "./reactions.tsx";

// The real actions run against the in-memory mock backend through stubbed fetch.
let network: MockNetwork;

const ROOM = SEED_IDS.rooms.quiet;

const VIEWER = SEED_IDS.viewer;

let posted = 0;

function headers() {
  return { "Content-Type": "application/json", "X-CSRF-Token": network.server.csrfToken() };
}

/** Posts a message as the viewer and puts it in the store, as the timeline would have it. */
async function post(markdown: string): Promise<MessageDTO> {
  posted += 1;

  const response = await fetch(`/api/v1/rooms/${ROOM}/messages`, {
    method: "POST",
    headers: headers(),
    body: JSON.stringify({
      clientMessageId: `components-${posted}`,
      markdownSource: markdown,
      replyToMessageId: null,
      replyNotifyAuthor: null,
    }),
  });

  const message: MessageDTO = await response.json();

  mutations.applyPage(ROOM, pageFixture([message]), "replace");

  return message;
}

function current(id: number): MessageDTO | undefined {
  return store.getState().messages[id];
}

beforeAll(() => {
  network = installMockNetwork();

  const meta = document.createElement("meta");

  meta.name = "csrf-token";
  meta.content = network.server.csrfToken();
  document.head.append(meta);

  // jsdom doesn't lay out, so it has no scrollIntoView; an opened editor scrolls itself into view.
  Element.prototype.scrollIntoView = () => undefined;
});

afterAll(() => network.restore());

beforeEach(async () => {
  // jsdom has no matchMedia; the motion helpers ask it about reduced motion.
  window.matchMedia = (query: string) => {
    const list = new EventTarget();

    return Object.assign(list, {
      matches: false,
      media: query,
      onchange: null,
      addListener: () => undefined,
      removeListener: () => undefined,
    });
  };

  const me: Me = await (await fetch("/api/v1/me")).json();

  mutations.reset();
  mutations.setMe(me);
  mutations.mergeUsers([userFixture(2, "Maya Okafor"), userFixture(3, "Theo Brandt")]);
  resetRecentEmoji();
  stopEditing();
});

describe("reaction pills", () => {
  const message = messageFixture(10, ROOM, {
    reactions: [
      { content: "🎉", title: "Party popper", imageUrl: null, reactorIds: [2, VIEWER] },
      { content: "🔥", title: "Fire", imageUrl: null, reactorIds: [3] },
    ],
    boosts: [
      { id: 1, boosterId: 2, content: "nice work", createdAt: "2026-10-06T00:00:00.000Z" },
      { id: 2, boosterId: VIEWER, content: "ship it", createdAt: "2026-10-06T00:00:01.000Z" },
    ],
  });

  it("shows counts, highlights the viewer's, and lists who reacted", () => {
    render(<ReactionsRow message={message} viewerId={VIEWER} canReact onAddReaction={() => {}} />);

    const party = screen.getByRole("button", {
      name: "Party popper: 2 reactions, including yours",
    });

    const fire = screen.getByRole("button", { name: "Fire: 1 reaction" });

    expect(party.getAttribute("aria-pressed")).toBe("true");
    expect(fire.getAttribute("aria-pressed")).toBe("false");
    expect(party.textContent).toContain("2");
  });

  it("toggles a reaction on click", async () => {
    const user = userEvent.setup();
    const sent = await post("Launch is on");
    const shown = { ...sent, reactions: message.reactions };

    render(<ReactionsRow message={shown} viewerId={VIEWER} canReact onAddReaction={() => {}} />);
    await user.click(screen.getByRole("button", { name: "Fire: 1 reaction" }));

    await waitFor(() =>
      expect(
        current(sent.id)?.reactions.find((entry) => entry.content === "🔥")?.reactorIds,
      ).toContain(VIEWER),
    );
  });

  it("opens the picker from the add pill", async () => {
    const user = userEvent.setup();
    const added: HTMLElement[] = [];

    render(
      <ReactionsRow
        message={message}
        viewerId={VIEWER}
        canReact
        onAddReaction={(anchor) => added.push(anchor)}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Add reaction" }));

    expect(added).toHaveLength(1);
  });

  it("lets the viewer take back their own boost, and only theirs", async () => {
    const user = userEvent.setup();
    const sent = await post("Beta is out");

    await fetch(`/api/v1/messages/${sent.id}/boosts`, {
      method: "POST",
      headers: headers(),
      body: JSON.stringify({ content: "ship it" }),
    });

    const boosted = await post("Beta is out, again");

    await fetch(`/api/v1/messages/${boosted.id}/boosts`, {
      method: "POST",
      headers: headers(),
      body: JSON.stringify({ content: "ship it" }),
    });

    const page = await (await fetch(`/api/v1/rooms/${ROOM}/messages`)).json();

    mutations.applyPage(ROOM, page, "replace");

    const withBoost = current(boosted.id);

    expect(withBoost?.boosts).toHaveLength(1);
    render(
      <ReactionsRow
        message={{
          ...(withBoost ?? boosted),
          boosts: [...message.boosts.slice(0, 1), ...(withBoost?.boosts ?? [])],
        }}
        viewerId={VIEWER}
        canReact
        onAddReaction={() => {}}
      />,
    );

    expect(screen.queryByRole("button", { name: /nice work/ })).toBeNull();
    await user.click(screen.getByRole("button", { name: "Remove your boost “ship it”" }));
    await waitFor(() => expect(current(boosted.id)?.boosts).toHaveLength(0));
  });

  it("renders nothing without reactions or boosts, and no add pill when reacting isn't allowed", () => {
    const { container, rerender } = render(
      <ReactionsRow
        message={messageFixture(11, ROOM)}
        viewerId={VIEWER}
        canReact
        onAddReaction={() => {}}
      />,
    );

    expect(container.childElementCount).toBe(0);

    rerender(
      <ReactionsRow
        message={message}
        viewerId={VIEWER}
        canReact={false}
        onAddReaction={() => {}}
      />,
    );
    expect(screen.queryByRole("button", { name: "Add reaction" })).toBeNull();
  });
});

describe("editing in place", () => {
  it("decides Enter, Shift+Enter and Esc like the composer", () => {
    const key = (name: string, change = {}) => ({
      key: name,
      shiftKey: false,
      altKey: false,
      metaKey: false,
      ctrlKey: false,
      ...change,
    });

    expect(editorKeyAction(key("Enter"))).toBe("save");
    expect(editorKeyAction(key("Enter", { shiftKey: true }))).toBeNull();
    expect(editorKeyAction(key("Enter", { altKey: true }))).toBeNull();
    expect(editorKeyAction(key("Enter", { metaKey: true }))).toBe("save");
    expect(editorKeyAction(key("Escape"))).toBe("cancel");
    expect(editorKeyAction(key("a"))).toBeNull();
  });

  it("saves the edited Markdown with Enter", async () => {
    const user = userEvent.setup();
    let closed = 0;
    const message = await post("Hello");

    render(
      <MessageEditor message={message} onClose={() => (closed += 1)} onRequestDelete={() => {}} />,
    );

    const box = screen.getByRole("textbox", { name: "Edit message" });

    expect(document.activeElement).toBe(box);
    await user.type(box, " there{Enter}");

    await waitFor(() => expect(closed).toBe(1));
    expect(current(message.id)?.markdownSource).toBe("Hello there");
    expect(current(message.id)?.editedAt).not.toBeNull();
  });

  it("closes without saving on Esc or on unchanged text, and asks before blanking", async () => {
    const user = userEvent.setup();
    let closed = 0;
    let deleteAsked = 0;
    const message = messageFixture(21, ROOM, { creatorId: VIEWER, markdownSource: "Keep" });

    render(
      <MessageEditor
        message={message}
        onClose={() => (closed += 1)}
        onRequestDelete={() => (deleteAsked += 1)}
      />,
    );

    const box = screen.getByRole("textbox", { name: "Edit message" });

    await user.type(box, "{Enter}");
    expect(closed).toBe(1);

    await user.type(box, "{Escape}");
    expect(closed).toBe(2);

    await user.clear(box);
    await user.type(box, "{Enter}");
    expect(deleteAsked).toBe(1);
    expect(closed).toBe(2);
  });

  it("loads the source of a rich-text-only message first", async () => {
    const sent = await post("From the server");
    const message = { ...sent, markdownSource: null };

    render(<MessageEditor message={message} onClose={() => {}} onRequestDelete={() => {}} />);

    expect(screen.getByRole("img", { name: "Loading the message" })).toBeTruthy();
    const box = await screen.findByRole("textbox", { name: "Edit message" });

    expect(box instanceof HTMLTextAreaElement ? box.value : null).toBe("From the server");
  });

  it("finds the viewer's newest editable message and opens it for the composer", () => {
    mutations.applyPage(
      ROOM,
      pageFixture([
        messageFixture(30, ROOM, { creatorId: VIEWER }),
        messageFixture(31, ROOM, { creatorId: VIEWER, systemNote: true }),
        messageFixture(32, ROOM, { creatorId: 2 }),
      ]),
      "replace",
    );

    const composer = document.createElement("textarea");

    document.body.append(composer);
    composer.focus();

    expect(lastEditableMessage(store.getState(), ROOM, null)?.id).toBe(30);
    act(() => {
      expect(editLastOwnMessage(ROOM)).toBe(true);
    });
    expect(editingId()).toBe(30);
    expect(stopEditing()).toBe(composer);
    expect(editLastOwnMessage(999)).toBe(false);
    composer.remove();
  });
});
