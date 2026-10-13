import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { messageFixture } from "../../api/testing.ts";
import type { GifItem } from "../../gen/GifItem.ts";
import type { GifList } from "../../gen/GifList.ts";
import { emptyTimeline, initialState } from "../../store/state.ts";
import { store } from "../../store/store.ts";
import { composerActions } from "../../sync/composer-actions.ts";
import { gifActions } from "../../sync/gif-actions.ts";
import { actions } from "../../sync/runtime.ts";
import { toastSnapshot } from "../../ui/toast-store.ts";
import { clearCommandCache } from "./autocomplete/suggestions.ts";
import { Composer } from "./composer.tsx";
import { draftKey, readDraft, writeDraft } from "./draft.ts";
import { ComposerGifButton, GIF_SEARCH_DEBOUNCE_MS } from "./gif-picker.tsx";
import { distributeGifs, moveGifFocus, resetGifConfig } from "./gif-store.ts";
import { resetStickers } from "./sticker-store.ts";

const ROOM = 12;

function gif(slug: string, title: string, width = 200, height = 150): GifItem {
  return {
    slug,
    klipyId: slug,
    title,
    attribution: "Powered by KLIPY",
    previewUrl: `/api/v1/gifs/media?u=${slug}.gif`,
    stillUrl: `/api/v1/gifs/media?u=${slug}.png`,
    width,
    height,
  };
}

const WAVE = gif("hello-wave", "Hello Wave", 200, 300);

const CHEER = gif("celebration-cheer", "Celebration Cheer", 300, 200);

const DANCE = gif("dance-party", "Dance Party", 200, 200);

const CAT = gif("cat-yawn", "Cat Yawn", 240, 180);

function list(items: readonly GifItem[], page = 1, hasNext = false): GifList {
  return { items: [...items], page, hasNext };
}

/** The IntersectionObserver callback the picker last registered, for the pagination test. */
let observe: IntersectionObserverCallback | null = null;

beforeEach(() => {
  sessionStorage.clear();
  clearCommandCache();
  resetStickers();
  resetGifConfig();
  observe = null;
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
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  );
  vi.stubGlobal(
    "IntersectionObserver",
    class {
      constructor(callback: IntersectionObserverCallback) {
        observe = callback;
      }

      observe() {}
      unobserve() {}
      disconnect() {}
    },
  );
  vi.spyOn(actions, "noteActivity").mockImplementation(() => undefined);
  vi.spyOn(actions, "setTyping").mockImplementation(() => undefined);
  vi.spyOn(actions.scheduled, "load").mockResolvedValue(undefined);
  vi.spyOn(composerActions, "slashCommands").mockResolvedValue([]);
  vi.spyOn(composerActions, "stickerCatalog").mockResolvedValue({ stickers: [], capacity: 60 });
});

afterEach(() => {
  cleanup();
  delete document.documentElement.dataset.motion;
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  store.setState(initialState, true);
});

/** Stubs every GIF read the picker makes; each test overrides the ones it drives. */
function stubReads() {
  const config = vi.spyOn(gifActions, "config").mockResolvedValue({ enabled: true });
  const trending = vi.spyOn(gifActions, "trending").mockResolvedValue(list([WAVE, CHEER]));
  const search = vi.spyOn(gifActions, "search").mockResolvedValue(list([WAVE]));

  const categories = vi.spyOn(gifActions, "categories").mockResolvedValue({
    categories: [{ name: "Hello Wave", query: "Hello Wave", previewUrl: WAVE.stillUrl }],
  });

  const autocomplete = vi
    .spyOn(gifActions, "autocomplete")
    .mockResolvedValue({ suggestions: ["hello", "hello there"] });

  vi.spyOn(gifActions, "suggestions").mockResolvedValue({ suggestions: [] });

  return { config, trending, search, categories, autocomplete };
}

/** Opens the picker from the composer's GIF button. */
async function openPicker() {
  fireEvent.click(await screen.findByRole("button", { name: "GIF" }));

  return screen.findByRole("dialog", { name: "Send a GIF" });
}

const names = (picker: HTMLElement) =>
  within(picker)
    .queryAllByRole("option")
    .map((option) => option.getAttribute("aria-label"));

/** Tells the picker's paging sentinel it came into view, through the stubbed observer. */
async function intersect() {
  await act(async () => {
    // SAFETY: the stub above records the callback; only isIntersecting is read, and no observer.
    observe?.([{ isIntersecting: true } as IntersectionObserverEntry], {} as IntersectionObserver);
  });
}

describe("the GIF columns", () => {
  it("balances the masonry columns by each GIF's height for its width", () => {
    const columns = distributeGifs([WAVE, CHEER, DANCE, CAT], 2);

    // WAVE is tall (300/200) and sits alone in its column first; the rest fill the shorter side.
    expect(columns.map((column) => column.map((item) => item.slug))).toEqual([
      ["hello-wave", "cat-yawn"],
      ["celebration-cheer", "dance-party"],
    ]);
  });

  it("moves the focus down a column and across to the next one", () => {
    const columns = distributeGifs([WAVE, CHEER, DANCE, CAT], 2);

    expect(moveGifFocus(columns, "hello-wave", "ArrowDown")).toBe("cat-yawn");
    expect(moveGifFocus(columns, "hello-wave", "ArrowRight")).toBe("celebration-cheer");
    expect(moveGifFocus(columns, "celebration-cheer", "ArrowLeft")).toBe("hello-wave");
    expect(moveGifFocus(columns, "cat-yawn", "ArrowUp")).toBe("hello-wave");
    expect(moveGifFocus(columns, "cat-yawn", "ArrowDown")).toBe("cat-yawn");
    expect(moveGifFocus(columns, "hello-wave", "Tab")).toBeNull();
  });
});

describe("the GIF button", () => {
  it("doesn't render at all when the feature is off", async () => {
    const { config } = stubReads();

    config.mockResolvedValue({ enabled: false });
    render(<ComposerGifButton onPick={async () => {}} />);

    await waitFor(() => expect(config).toHaveBeenCalled());
    expect(screen.queryByRole("button", { name: "GIF" })).toBeNull();
  });

  it("shows trending GIFs and categories, and a category runs its search", async () => {
    const { trending, search } = stubReads();

    render(<ComposerGifButton onPick={async () => {}} />);

    const picker = await openPicker();

    await waitFor(() => expect(names(picker)).toEqual(["Hello Wave", "Celebration Cheer"]));
    expect(trending).toHaveBeenCalledWith(1);
    expect(within(picker).getByText("Powered by KLIPY")).toBeTruthy();

    fireEvent.click(within(picker).getByRole("button", { name: "Hello Wave" }));

    await waitFor(() => expect(search).toHaveBeenCalledWith("Hello Wave", 1));
    await waitFor(() => expect(names(picker)).toEqual(["Hello Wave"]));
  });

  it("debounces the search and offers autocomplete terms under the field", async () => {
    const { search, autocomplete } = stubReads();

    render(<ComposerGifButton onPick={async () => {}} />);

    const picker = await openPicker();
    const field = within(picker).getByRole("combobox", { name: "Search GIFs" });

    await waitFor(() => expect(names(picker)).toEqual(["Hello Wave", "Celebration Cheer"]));

    fireEvent.change(field, { target: { value: "h" } });
    fireEvent.change(field, { target: { value: "he" } });
    fireEvent.change(field, { target: { value: "hel" } });
    expect(search).not.toHaveBeenCalled();

    await waitFor(() => expect(search).toHaveBeenCalledExactlyOnceWith("hel", 1), {
      timeout: GIF_SEARCH_DEBOUNCE_MS + 2000,
    });
    expect(autocomplete).toHaveBeenCalledWith("hel");
    expect(await within(picker).findByRole("button", { name: "hello there" })).toBeTruthy();

    fireEvent.click(within(picker).getByRole("button", { name: "hello there" }));
    await waitFor(() => expect(search).toHaveBeenCalledWith("hello there", 1));
    expect(field).toHaveProperty("value", "hello there");
  });

  it("pages in more results when the end of the grid comes into view", async () => {
    const { trending } = stubReads();

    trending
      .mockResolvedValueOnce(list([WAVE, CHEER], 1, true))
      .mockResolvedValueOnce(list([DANCE], 2, false));
    render(<ComposerGifButton onPick={async () => {}} />);

    const picker = await openPicker();

    await waitFor(() => expect(names(picker)).toEqual(["Hello Wave", "Celebration Cheer"]));
    expect(observe).not.toBeNull();

    await act(async () => {
      // SAFETY: the stub above records the callback; only isIntersecting is read, and no observer.
      observe?.(
        [{ isIntersecting: true } as IntersectionObserverEntry],
        {} as IntersectionObserver,
      );
    });

    await waitFor(() => expect(trending).toHaveBeenCalledWith(2));
    await waitFor(() =>
      expect(names(picker)).toEqual(["Hello Wave", "Celebration Cheer", "Dance Party"]),
    );
  });

  it("pages again after the search changes while a page is loading", async () => {
    const { trending, search } = stubReads();

    let finishPageTwo: (() => void) | null = null;

    trending.mockResolvedValueOnce(list([WAVE, CHEER], 1, true)).mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          finishPageTwo = () => resolve(list([DANCE], 2, false));
        }),
    );
    search
      .mockResolvedValueOnce(list([CAT], 1, true))
      .mockResolvedValueOnce(list([WAVE], 2, false));
    render(<ComposerGifButton onPick={async () => {}} />);

    const picker = await openPicker();

    await waitFor(() => expect(names(picker)).toEqual(["Hello Wave", "Celebration Cheer"]));

    // Start page two, then change the search before it lands.
    await intersect();
    await waitFor(() => expect(within(picker).getByText("Loading more GIFs…")).toBeTruthy());

    fireEvent.change(within(picker).getByRole("combobox", { name: "Search GIFs" }), {
      target: { value: "cat" },
    });

    await waitFor(() => expect(names(picker)).toEqual(["Cat Yawn"]), {
      timeout: GIF_SEARCH_DEBOUNCE_MS + 2000,
    });

    // The stale page lands late: it must not stick the new results on "Loading more".
    await act(async () => finishPageTwo?.());
    expect(within(picker).queryByText("Loading more GIFs…")).toBeNull();
    expect(names(picker)).toEqual(["Cat Yawn"]);

    // Paging is armed again for the new query.
    await intersect();
    await waitFor(() => expect(search).toHaveBeenCalledWith("cat", 2));
    await waitFor(() => expect(names(picker)).toEqual(["Cat Yawn", "Hello Wave"]));
  });

  it("shows a placeholder, never the animated preview, when a GIF has no still", async () => {
    const { trending } = stubReads();

    const noStill: GifItem = { ...CAT, stillUrl: null };

    trending.mockResolvedValue(list([WAVE, noStill]));
    render(<ComposerGifButton onPick={async () => {}} />);

    const picker = await openPicker();

    await waitFor(() => expect(names(picker)).toEqual(["Hello Wave", "Cat Yawn"]));

    const cat = () => within(picker).getByRole("option", { name: "Cat Yawn" });

    const preview = () =>
      [...picker.querySelectorAll("img")].some(
        (image) => image.getAttribute("src") === noStill.previewUrl,
      );

    // At rest there is only its titled placeholder: the animated preview never loads.
    expect(cat().querySelector("img")).toBeNull();
    expect(cat().textContent).toContain("Cat Yawn");
    expect(preview()).toBe(false);

    // Hovering (the active GIF) is the only time it animates.
    fireEvent.pointerMove(cat());
    expect(cat().querySelector("img")?.getAttribute("src")).toBe(noStill.previewUrl);

    // Under reduced motion even the active GIF stays on the placeholder.
    document.documentElement.dataset.motion = "reduce";
    await waitFor(() => expect(cat().querySelector("img")).toBeNull());
    expect(preview()).toBe(false);
  });

  it("moves through the grid with the arrows and sends with Enter", async () => {
    stubReads();

    const pick = vi.fn().mockResolvedValue(undefined);

    render(<ComposerGifButton onPick={pick} />);

    const picker = await openPicker();
    const field = within(picker).getByRole("combobox", { name: "Search GIFs" });

    await waitFor(() => expect(names(picker)).toEqual(["Hello Wave", "Celebration Cheer"]));
    expect(document.activeElement).toBe(field);

    // WAVE stands alone first in its column; Right crosses to CHEER's column.
    fireEvent.keyDown(field, { key: "ArrowRight" });
    expect(field.getAttribute("aria-activedescendant")).toBe(
      within(picker).getByRole("option", { name: "Celebration Cheer" }).id,
    );

    fireEvent.keyDown(field, { key: "Enter" });
    await waitFor(() => expect(pick).toHaveBeenCalledExactlyOnceWith(CHEER));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  it("rests on the still until hovered, and always under reduced motion", async () => {
    stubReads();
    render(<ComposerGifButton onPick={async () => {}} />);

    const picker = await openPicker();

    await waitFor(() => expect(names(picker)).toEqual(["Hello Wave", "Celebration Cheer"]));

    const wave = () =>
      within(picker).getByRole("option", { name: "Hello Wave" }).querySelector("img");

    const cheer = () =>
      within(picker).getByRole("option", { name: "Celebration Cheer" }).querySelector("img");

    // The active GIF plays; the resting one shows its still.
    expect(wave()?.getAttribute("src")).toBe(WAVE.previewUrl);
    expect(wave()?.getAttribute("alt")).toBe("Hello Wave");
    expect(cheer()?.getAttribute("src")).toBe(CHEER.stillUrl);

    fireEvent.pointerMove(within(picker).getByRole("option", { name: "Celebration Cheer" }));
    expect(cheer()?.getAttribute("src")).toBe(CHEER.previewUrl);
    expect(wave()?.getAttribute("src")).toBe(WAVE.stillUrl);

    document.documentElement.dataset.motion = "reduce";
    await waitFor(() => expect(cheer()?.getAttribute("src")).toBe(CHEER.stillUrl));
  });

  it("says when nothing is found", async () => {
    const { search } = stubReads();

    search.mockResolvedValue(list([]));
    render(<ComposerGifButton onPick={async () => {}} />);

    const picker = await openPicker();

    fireEvent.change(within(picker).getByRole("combobox", { name: "Search GIFs" }), {
      target: { value: "zebra" },
    });

    expect(await within(picker).findByText("No GIFs found")).toBeTruthy();
  });

  it("offers a retry when the load fails", async () => {
    const { trending } = stubReads();

    trending.mockRejectedValueOnce(new Error("Service Unavailable"));
    render(<ComposerGifButton onPick={async () => {}} />);

    const picker = await openPicker();
    const retry = await within(picker).findByRole("button", { name: "Retry" });

    fireEvent.click(retry);

    await waitFor(() => expect(names(picker)).toEqual(["Hello Wave", "Celebration Cheer"]));
  });
});

describe("sending a GIF from the composer", () => {
  it("sends at once with the draft as its text, and clears the draft", async () => {
    stubReads();

    const send = vi.spyOn(gifActions, "send").mockResolvedValue(messageFixture(20, ROOM));
    const key = draftKey(ROOM, null);

    writeDraft(key, "look at this");
    render(<Composer roomId={ROOM} />);

    const picker = await openPicker();

    await waitFor(() => expect(names(picker)).toEqual(["Hello Wave", "Celebration Cheer"]));
    fireEvent.click(within(picker).getByRole("option", { name: "Hello Wave" }));

    await waitFor(() =>
      expect(send).toHaveBeenCalledExactlyOnceWith({
        roomId: ROOM,
        slug: "hello-wave",
        clientMessageId: expect.any(String),
        markdownSource: "look at this",
      }),
    );
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(readDraft(key)).toBe("");
    expect(screen.getByRole("textbox", { name: /Message/ })).toHaveProperty("value", "");
  });

  it("sends into the thread a thread's composer is for", async () => {
    stubReads();

    const send = vi.spyOn(gifActions, "send").mockResolvedValue(messageFixture(21, ROOM));

    render(<Composer roomId={ROOM} threadId={7} placeholder="Reply…" />);

    const picker = await openPicker();

    await waitFor(() => expect(names(picker)).toEqual(["Hello Wave", "Celebration Cheer"]));
    fireEvent.click(within(picker).getByRole("option", { name: "Celebration Cheer" }));

    await waitFor(() =>
      expect(send).toHaveBeenCalledWith(
        expect.objectContaining({ roomId: ROOM, threadId: 7, slug: "celebration-cheer" }),
      ),
    );
  });

  it("keeps the picker open and the draft when the send fails", async () => {
    stubReads();
    vi.spyOn(gifActions, "send").mockRejectedValue(new Error("Service Unavailable"));

    const key = draftKey(ROOM, null);

    writeDraft(key, "still typing");
    render(<Composer roomId={ROOM} />);

    const picker = await openPicker();

    await waitFor(() => expect(names(picker)).toEqual(["Hello Wave", "Celebration Cheer"]));
    fireEvent.click(within(picker).getByRole("option", { name: "Hello Wave" }));

    await waitFor(() => expect(toastSnapshot().at(-1)?.tone).toBe("danger"));
    expect(screen.getByRole("dialog", { name: "Send a GIF" })).toBe(picker);
    expect(readDraft(key)).toBe("still typing");
  });

  it("starts a new thread with the GIF as its first reply", async () => {
    stubReads();

    const submit = vi.fn().mockResolvedValue(undefined);

    render(<Composer roomId={ROOM} onSubmit={submit} placeholder="Reply…" />);

    const picker = await openPicker();

    await waitFor(() => expect(names(picker)).toEqual(["Hello Wave", "Celebration Cheer"]));
    fireEvent.click(within(picker).getByRole("option", { name: "Hello Wave" }));

    // The server counts the GIF as one file and wants one spoiler flag per file
    // (`file_spoilers` in crates/api): an empty list is refused, `[false]` is the GIF's flag.
    await waitFor(() =>
      expect(submit).toHaveBeenCalledExactlyOnceWith({
        markdown: "",
        attachmentSignedId: null,
        attachmentSignedIds: [],
        attachmentSpoilers: [false],
        driveFileIds: [],
        gifSlug: "hello-wave",
        stickerId: null,
      }),
    );
  });

  it("keeps a newer draft when a GIF send lands after leaving and coming back", async () => {
    stubReads();

    let finishSend: (() => void) | null = null;

    vi.spyOn(gifActions, "send").mockImplementation(
      () =>
        new Promise((resolve) => {
          finishSend = () => resolve(messageFixture(22, ROOM));
        }),
    );

    const key = draftKey(ROOM, null);

    writeDraft(key, "look at this");

    const first = render(<Composer roomId={ROOM} />);

    const picker = await openPicker();

    await waitFor(() => expect(names(picker)).toEqual(["Hello Wave", "Celebration Cheer"]));
    fireEvent.click(within(picker).getByRole("option", { name: "Hello Wave" }));

    // Leave the room and come back while the send is still out, then keep typing there.
    first.unmount();
    cleanup();
    render(<Composer roomId={ROOM} />);

    const box = screen.getByRole("textbox", { name: /Message/ });

    fireEvent.change(box, { target: { value: "newer words" } });
    expect(readDraft(key)).toBe("newer words");

    await act(async () => finishSend?.());

    expect(readDraft(key)).toBe("newer words");
    expect(box).toHaveProperty("value", "newer words");
  });
});
