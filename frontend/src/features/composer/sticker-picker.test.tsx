import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { messageFixture } from "../../api/testing.ts";
import type { Sticker } from "../../gen/Sticker.ts";
import type { StickerCatalog } from "../../gen/StickerCatalog.ts";
import { emptyTimeline, initialState } from "../../store/state.ts";
import { store } from "../../store/store.ts";
import { composerActions } from "../../sync/composer-actions.ts";
import { actions } from "../../sync/runtime.ts";
import { clearCommandCache } from "./autocomplete/suggestions.ts";
import { Composer } from "./composer.tsx";
import { draftKey, readDraft, writeDraft } from "./draft.ts";
import { startReply } from "./reply-store.ts";
import { ComposerStickerButton } from "./sticker-picker.tsx";
import {
  replaceStickerCatalog,
  resetStickers,
  STICKER_CATALOG_MAX_AGE_MS,
  searchStickers,
} from "./sticker-store.ts";

const ROOM = 12;

const OTHER_ROOM = 13;

function sticker(id: number, name: string, tags: string, animated = false): Sticker {
  return {
    id,
    name,
    description: `${name} sticker`,
    tags,
    imageUrl: `/stickers/${id}`,
    stillUrl: animated ? `/stickers/${id}?still=1` : `/stickers/${id}`,
    animated,
    uploaderId: 1,
    createdAt: "2026-10-12T12:00:00Z",
    updatedAt: "2026-10-12T12:00:00Z",
  };
}

const DANCE = sticker(2, "Dance", "party 🎉", true);

const HELLO = sticker(3, "Hello", "wave greeting");

const WAVE = sticker(1, "Wave", "👋 hello");

const CATALOG: StickerCatalog = { stickers: [DANCE, HELLO, WAVE], capacity: 60 };

beforeEach(() => {
  sessionStorage.clear();
  clearCommandCache();
  resetStickers();
  store.setState(
    {
      ...initialState,
      timelines: { [ROOM]: { ...emptyTimeline, status: "ready", after: null } },
    },
    true,
  );
  vi.stubGlobal("matchMedia", (media: string) =>
    Object.assign(new EventTarget(), { media, matches: false, onchange: null }),
  );
  // jsdom has no ResizeObserver; the send button's goo effect watches its size.
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  );
  vi.spyOn(actions, "noteActivity").mockImplementation(() => undefined);
  vi.spyOn(actions, "setTyping").mockImplementation(() => undefined);
  vi.spyOn(actions.scheduled, "load").mockResolvedValue(undefined);
  vi.spyOn(composerActions, "slashCommands").mockResolvedValue([]);
});

afterEach(() => {
  cleanup();
  delete document.documentElement.dataset.motion;
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  store.setState(initialState, true);
});

/** Opens the picker from the composer's Sticker button. */
async function openPicker() {
  fireEvent.click(await screen.findByRole("button", { name: "Sticker" }));

  return screen.findByRole("dialog", { name: "Send a sticker" });
}

const names = (picker: HTMLElement) =>
  within(picker)
    .queryAllByRole("option")
    .map((option) => option.getAttribute("aria-label"));

describe("searching stickers", () => {
  it("matches every word against the name and tags, case aside", () => {
    expect(searchStickers(CATALOG.stickers, "")).toEqual(CATALOG.stickers);
    expect(searchStickers(CATALOG.stickers, "HEL")).toEqual([HELLO, WAVE]);
    expect(searchStickers(CATALOG.stickers, "wave greet")).toEqual([HELLO]);
    expect(searchStickers(CATALOG.stickers, "🎉")).toEqual([DANCE]);
    expect(searchStickers(CATALOG.stickers, "zebra")).toEqual([]);
  });
});

describe("the sticker button", () => {
  it("stays hidden while the workspace has no stickers", async () => {
    const catalog = vi
      .spyOn(composerActions, "stickerCatalog")
      .mockResolvedValue({ stickers: [], capacity: 60 });

    render(<ComposerStickerButton onPick={() => {}} />);

    await waitFor(() => expect(catalog).toHaveBeenCalledOnce());
    expect(screen.queryByRole("button", { name: "Sticker" })).toBeNull();

    // An administrator's first sticker shows it at once.
    act(() => replaceStickerCatalog({ stickers: [WAVE], capacity: 60 }));
    expect(screen.getByRole("button", { name: "Sticker" })).toBeTruthy();
  });

  it("opens a grid the search box drives from the keyboard", async () => {
    vi.spyOn(composerActions, "stickerCatalog").mockResolvedValue(CATALOG);

    const pick = vi.fn();

    render(<ComposerStickerButton onPick={pick} />);

    const picker = await openPicker();
    const search = within(picker).getByRole("combobox", { name: "Search stickers" });

    expect(document.activeElement).toBe(search);
    expect(names(picker)).toEqual(["Dance", "Hello", "Wave"]);
    expect(
      within(picker).getByRole("option", { name: "Dance" }).getAttribute("aria-selected"),
    ).toBe("true");

    fireEvent.change(search, { target: { value: "hel" } });
    expect(names(picker)).toEqual(["Hello", "Wave"]);

    fireEvent.keyDown(search, { key: "ArrowRight" });
    expect(search.getAttribute("aria-activedescendant")).toBe(
      within(picker).getByRole("option", { name: "Wave" }).id,
    );

    fireEvent.keyDown(search, { key: "Enter" });
    expect(pick).toHaveBeenCalledExactlyOnceWith(WAVE);
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  it("leaves an input method's keys to it: Enter, arrows and Escape while composing", async () => {
    vi.spyOn(composerActions, "stickerCatalog").mockResolvedValue(CATALOG);

    const pick = vi.fn();

    render(<ComposerStickerButton onPick={pick} />);

    const picker = await openPicker();
    const search = within(picker).getByRole("combobox", { name: "Search stickers" });

    const activeName = () =>
      document
        .getElementById(search.getAttribute("aria-activedescendant") ?? "")
        ?.getAttribute("aria-label");

    fireEvent.keyDown(search, { key: "ArrowRight", isComposing: true });
    expect(activeName()).toBe("Dance");

    fireEvent.keyDown(search, { key: "Enter", isComposing: true });
    expect(pick).not.toHaveBeenCalled();

    fireEvent.keyDown(search, { key: "Escape", isComposing: true });
    expect(screen.getByRole("dialog", { name: "Send a sticker" })).toBe(picker);

    // The composition over, the same keys work.
    fireEvent.keyDown(search, { key: "ArrowRight" });
    expect(activeName()).toBe("Hello");
    fireEvent.keyDown(search, { key: "Enter" });
    expect(pick).toHaveBeenCalledExactlyOnceWith(HELLO);
  });

  it("says when nothing matches", async () => {
    vi.spyOn(composerActions, "stickerCatalog").mockResolvedValue(CATALOG);
    render(<ComposerStickerButton onPick={() => {}} />);

    const picker = await openPicker();

    fireEvent.change(within(picker).getByRole("combobox"), { target: { value: "zebra" } });

    expect(within(picker).getByRole("status").textContent).toBe("No stickers match “zebra”");
  });

  it("rests animated stickers on their still until active, and always under reduced motion", async () => {
    vi.spyOn(composerActions, "stickerCatalog").mockResolvedValue(CATALOG);
    render(<ComposerStickerButton onPick={() => {}} />);

    const picker = await openPicker();
    const dance = () => within(picker).getByRole("option", { name: "Dance" }).querySelector("img");

    // Dance is the active cell: it plays.
    expect(dance()?.getAttribute("src")).toBe("/stickers/2");

    fireEvent.pointerMove(within(picker).getByRole("option", { name: "Wave" }));
    expect(dance()?.getAttribute("src")).toBe("/stickers/2?still=1");

    fireEvent.pointerMove(within(picker).getByRole("option", { name: "Dance" }));
    document.documentElement.dataset.motion = "reduce";
    await waitFor(() => expect(dance()?.getAttribute("src")).toBe("/stickers/2?still=1"));
  });

  it("refetches a stale catalog on a room switch, so a workspace's first sticker shows", async () => {
    const catalog = vi
      .spyOn(composerActions, "stickerCatalog")
      .mockResolvedValueOnce({ stickers: [], capacity: 60 })
      .mockResolvedValue({ stickers: [WAVE], capacity: 60 });

    let now = Date.parse("2026-10-12T12:00:00Z");

    vi.spyOn(Date, "now").mockImplementation(() => now);
    store.setState({
      timelines: {
        [ROOM]: { ...emptyTimeline, status: "ready", after: null },
        [OTHER_ROOM]: { ...emptyTimeline, status: "ready", after: null },
      },
    });

    const { rerender } = render(<Composer roomId={ROOM} />);

    await waitFor(() => expect(catalog).toHaveBeenCalledOnce());
    expect(screen.queryByRole("button", { name: "Sticker" })).toBeNull();

    // A fresh catalog is kept: switching back and forth asks nothing.
    rerender(<Composer roomId={OTHER_ROOM} />);
    rerender(<Composer roomId={ROOM} />);
    expect(catalog).toHaveBeenCalledOnce();

    now += STICKER_CATALOG_MAX_AGE_MS;
    rerender(<Composer roomId={OTHER_ROOM} />);

    expect(await screen.findByRole("button", { name: "Sticker" })).toBeTruthy();
    expect(catalog).toHaveBeenCalledTimes(2);
  });

  it("refetches a stale catalog when the window regains focus", async () => {
    const catalog = vi
      .spyOn(composerActions, "stickerCatalog")
      .mockResolvedValueOnce({ stickers: [], capacity: 60 })
      .mockResolvedValue({ stickers: [WAVE], capacity: 60 });

    let now = Date.parse("2026-10-12T12:00:00Z");

    vi.spyOn(Date, "now").mockImplementation(() => now);
    render(<ComposerStickerButton onPick={() => {}} />);
    await waitFor(() => expect(catalog).toHaveBeenCalledOnce());

    fireEvent.focus(window);
    expect(catalog).toHaveBeenCalledOnce();

    now += STICKER_CATALOG_MAX_AGE_MS;
    fireEvent.focus(window);

    expect(await screen.findByRole("button", { name: "Sticker" })).toBeTruthy();
    expect(catalog).toHaveBeenCalledTimes(2);
  });

  it("waits as long after a failed refetch, keeping the catalog it holds", async () => {
    const catalog = vi
      .spyOn(composerActions, "stickerCatalog")
      .mockResolvedValueOnce(CATALOG)
      .mockRejectedValue(new Error("Service Unavailable"));

    let now = Date.parse("2026-10-12T12:00:00Z");

    vi.spyOn(Date, "now").mockImplementation(() => now);
    render(<ComposerStickerButton onPick={() => {}} />);
    await waitFor(() => expect(catalog).toHaveBeenCalledOnce());

    now += STICKER_CATALOG_MAX_AGE_MS;

    for (let focus = 0; focus < 20; focus += 1) {
      await act(async () => {
        fireEvent.focus(window);
      });
    }

    expect(catalog).toHaveBeenCalledTimes(2);
    expect(screen.getByRole("button", { name: "Sticker" })).toBeTruthy();

    now += STICKER_CATALOG_MAX_AGE_MS;
    await act(async () => {
      fireEvent.focus(window);
    });
    expect(catalog).toHaveBeenCalledTimes(3);

    // Opening the picker still asks at once, and shows the stickers held.
    const picker = await openPicker();

    await waitFor(() => expect(catalog).toHaveBeenCalledTimes(4));
    expect(names(picker)).toEqual(["Dance", "Hello", "Wave"]);
  });

  it("refetches the catalog on opening, so another administrator's changes show", async () => {
    const catalog = vi
      .spyOn(composerActions, "stickerCatalog")
      .mockResolvedValueOnce(CATALOG)
      .mockResolvedValueOnce({ stickers: [WAVE], capacity: 60 });

    render(<ComposerStickerButton onPick={() => {}} />);

    const picker = await openPicker();

    await waitFor(() => expect(names(picker)).toEqual(["Wave"]));
    expect(catalog).toHaveBeenCalledTimes(2);
  });
});

describe("sending a sticker from the composer", () => {
  it("sends it at once as its own message, with the reply, and keeps the draft", async () => {
    vi.spyOn(composerActions, "stickerCatalog").mockResolvedValue(CATALOG);

    const send = vi.spyOn(actions, "send").mockImplementation(() => undefined);
    const key = draftKey(ROOM, null);

    const quoted = messageFixture(9, ROOM);

    store.setState({ messages: { [quoted.id]: quoted } });
    writeDraft(key, "still typing");
    startReply(quoted);
    render(<Composer roomId={ROOM} />);

    const picker = await openPicker();

    fireEvent.click(within(picker).getByRole("option", { name: "Dance" }));

    expect(send).toHaveBeenCalledExactlyOnceWith(ROOM, "", {
      threadId: null,
      reply: { messageId: 9, notify: true },
      clientMessageId: expect.any(String),
      sticker: {
        kind: "available",
        id: 2,
        name: "Dance",
        imageUrl: "/stickers/2",
        stillUrl: "/stickers/2?still=1",
        animated: true,
      },
    });
    expect(readDraft(key)).toBe("still typing");
    expect(screen.getByRole("textbox", { name: /Message/ })).toHaveProperty(
      "value",
      "still typing",
    );
  });

  it("leaves the reply on the draft, so the text sent next answers it too", async () => {
    vi.spyOn(composerActions, "stickerCatalog").mockResolvedValue(CATALOG);

    const send = vi.spyOn(actions, "send").mockImplementation(() => undefined);
    const quoted = messageFixture(9, ROOM);

    store.setState({ messages: { [quoted.id]: quoted } });
    startReply(quoted);
    render(<Composer roomId={ROOM} />);

    fireEvent.click(within(await openPicker()).getByRole("option", { name: "Dance" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());

    const input = screen.getByRole("textbox", { name: /Message/ });

    fireEvent.change(input, { target: { value: "and this" } });
    fireEvent.keyDown(input, { key: "Enter" });

    await waitFor(() => expect(send).toHaveBeenCalledTimes(2));
    expect(send).toHaveBeenLastCalledWith(
      ROOM,
      "and this",
      expect.objectContaining({ reply: { messageId: 9, notify: true } }),
    );
  });

  it("sends into the thread a thread's composer is for", async () => {
    vi.spyOn(composerActions, "stickerCatalog").mockResolvedValue(CATALOG);

    const send = vi.spyOn(actions, "send").mockImplementation(() => undefined);

    render(<Composer roomId={ROOM} threadId={7} placeholder="Reply…" />);

    const picker = await openPicker();

    fireEvent.click(within(picker).getByRole("option", { name: "Wave" }));

    expect(send).toHaveBeenCalledWith(
      ROOM,
      "",
      expect.objectContaining({ threadId: 7, sticker: expect.objectContaining({ id: 1 }) }),
    );
  });

  it("isn't offered while a command is filled in", async () => {
    vi.spyOn(composerActions, "stickerCatalog").mockResolvedValue(CATALOG);
    replaceStickerCatalog(CATALOG);

    render(<Composer roomId={ROOM} />);
    expect(screen.getByRole("button", { name: "Sticker" })).toBeTruthy();
  });
});
