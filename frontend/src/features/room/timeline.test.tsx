import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render } from "@testing-library/react";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { messageFixture } from "../../api/testing.ts";
import type { Poll } from "../../gen/Poll.ts";
import { parseBoardSearch } from "../../lib/board-search.ts";
import { resetReaderFocusForTests } from "../../lib/reader-focus.ts";
import type { Boot, Timeline as RoomTimeline } from "../../store/model.ts";
import { removeMessage } from "../../store/reducers.ts";
import { emptyTimeline, initialState } from "../../store/state.ts";
import { store } from "../../store/store.ts";
import { panes } from "../../sync/panes.ts";
import { actions } from "../../sync/runtime.ts";
import { Dialog } from "../../ui/dialog.tsx";
import { holdCardsChunk } from "../cards/card-slot.tsx";
import { openPane } from "../panes/pane-store.ts";
import { RightPane } from "../panes/right-pane.tsx";
import { Timeline } from "./timeline.tsx";
import { timelineItems } from "./timeline-items.ts";

const ROOM = 12;

const FOCUS = 3;

/** Frames the timeline waits for a viewport before it must release newer paging anyway. */
const ANCHOR_ATTEMPTS = 30;

/** How long a permalink may stay hidden on a window or cards that never arrive. */
const PLACEMENT_WAIT_MS = 2000;

/** Releases a cards-chunk hold started by the test that stalls settlement. */
let releaseCards: (() => void) | null = null;

const frames = new Map<number, FrameRequestCallback>();

let nextFrame = 1;

const scrolls: number[] = [];

function isScrollOptions(value: ScrollToOptions | number): value is ScrollToOptions {
  return Object(value) === value;
}

/** When set, a programmatic scroll notifies the list, so it mounts the centred row. */
let emitScroll = false;

function install(ids: readonly number[], patch: Partial<RoomTimeline> = {}) {
  const rows = ids.map((id) => messageFixture(id, ROOM));

  store.setState({
    messages: Object.fromEntries(rows.map((message) => [message.id, message])),
    timelines: {
      [ROOM]: {
        ...emptyTimeline,
        status: "ready",
        ids,
        before: 1,
        after: 99,
        generation: 1,
        ...patch,
      },
    },
  });
}

function patchTimeline(patch: Partial<RoomTimeline>) {
  const timeline = store.getState().timelines[ROOM] ?? emptyTimeline;

  store.setState({ timelines: { [ROOM]: { ...timeline, ...patch } } });
}

/** Where `focus` sits among the rows the timeline is placing. */
function rowIndex(focus: number): number {
  const timeline = store.getState().timelines[ROOM] ?? emptyTimeline;

  return timelineItems({
    timeline,
    messages: store.getState().messages,
    pending: [],
    now: Date.now(),
  }).findIndex((item) => item.kind === "message" && item.message.id === focus);
}

async function renderTimeline(focus: number | null) {
  const root = createRootRoute({
    component: () => <Timeline roomId={ROOM} focusMessageId={focus} />,
  });

  const router = createRouter({
    routeTree: root,
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());
}

/** The timeline plus a dialog whose close hands focus back to whatever opened it. */
let setDialogOpen: (open: boolean) => void = () => undefined;

function TimelineDialogHost({ focus }: { readonly focus: number }) {
  const [open, setOpen] = useState(false);

  setDialogOpen = setOpen;

  return (
    <>
      <Timeline roomId={ROOM} focusMessageId={focus} />
      <Dialog open={open} onOpenChange={setOpen} title="Confirm">
        <button type="button">Stay</button>
      </Dialog>
    </>
  );
}

async function renderTimelineDialog(focus: number) {
  const root = createRootRoute({
    component: () => <TimelineDialogHost focus={focus} />,
  });

  const router = createRouter({
    routeTree: root,
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());
}

/** The timeline beside the real right pane, so Escape can hand focus back to a row. */
function TimelinePaneHost({ focus }: { readonly focus: number }) {
  return (
    <>
      <Timeline roomId={ROOM} focusMessageId={focus} />
      <RightPane roomId={ROOM} />
    </>
  );
}

async function renderTimelinePane(focus: number) {
  const root = createRootRoute();

  const room = createRoute({
    getParentRoute: () => root,
    path: "/r/$roomId",
    component: () => <TimelinePaneHost focus={focus} />,
    validateSearch: parseBoardSearch,
    params: {
      parse: ({ roomId }) => ({ roomId: Number(roomId) }),
      stringify: ({ roomId }) => ({ roomId: String(roomId) }),
    },
  });

  const router = createRouter({
    routeTree: root.addChildren([room]),
    history: createMemoryHistory({ initialEntries: [`/r/${ROOM}`] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());
}

/** jsdom here has no `matchMedia`; the pane asks which layout it is before it can take focus. */
function installMatchMedia(): () => void {
  const previous = window.matchMedia;

  window.matchMedia = (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addEventListener: () => undefined,
    removeEventListener: () => undefined,
    dispatchEvent: () => false,
    addListener: () => undefined,
    removeListener: () => undefined,
  });

  return () => {
    window.matchMedia = previous;
  };
}

const VIEWER: Boot = {
  user: { id: 7, name: "Ada", avatarUrl: "/a" },
  account: {
    name: "Acme",
    logoUrl: null,
    logoStillUrl: null,
    bannerUrl: null,
    bannerStillUrl: null,
  },
  theme: "system",
  textSize: "default",
  cableUrl: "/cable",
  serviceWorkerUrl: null,
  version: "test",
  revision: null,
};

/** Where a virtualised row sits in the list, from the wrapper Virtua positions. */
function rowTop(row: Element): number {
  const top = Number.parseFloat(row.parentElement?.style.top ?? "");

  if (!Number.isFinite(top)) {
    throw new Error("The row has no virtual offset");
  }

  return top;
}

function listViewportHeight(list: HTMLElement, fallback: number): number {
  return list.clientHeight > 0 ? list.clientHeight : fallback;
}

async function settlePlacement() {
  for (let frame = 0; frame < ANCHOR_ATTEMPTS && frames.size > 0; frame += 1) {
    await flushFrame();
  }
}

function replaceAround(ids: readonly number[]) {
  store.setState({
    messages: {
      ...store.getState().messages,
      ...Object.fromEntries(ids.map((id) => [id, messageFixture(id, ROOM)])),
    },
  });
  patchTimeline({ ids, before: 9, after: 13, generation: 2 });
}

async function flushFrame() {
  const batch = [...frames.entries()];

  frames.clear();
  await act(async () => {
    for (const [, callback] of batch) {
      callback(0);
    }
  });
}

/** A ResizeObserver jsdom can construct. It never reports a size, so the list stays unmeasured. */
class SilentObserver implements ResizeObserver {
  private readonly callback: ResizeObserverCallback;

  constructor(callback: ResizeObserverCallback) {
    this.callback = callback;
  }

  observe(target: Element): void {
    target.hasAttribute("data-message-row");
  }

  unobserve(target: Element): void {
    target.hasAttribute("data-message-row");
  }

  disconnect(): void {
    this.callback.length;
  }
}

/**
 * Reports a viewport and row sizes, which jsdom never lays out. Item measurement in the list
 * ignores elements whose `offsetParent` is null, so rows report one. The callback is deferred so
 * it does not flush React from inside the list's own layout effect.
 */
function measureList(row = 48, boxHeight = 360): () => void {
  const previousObserver = globalThis.ResizeObserver;

  const previousOffsetParent = Object.getOwnPropertyDescriptor(
    HTMLElement.prototype,
    "offsetParent",
  );

  class Observer implements ResizeObserver {
    private readonly notify: ResizeObserverCallback;

    constructor(notify: ResizeObserverCallback) {
      this.notify = notify;
    }

    observe(target: Element): void {
      const height = target.hasAttribute("data-message-row") ? row : boxHeight;
      const box = DOMRectReadOnly.fromRect({ width: 320, height });

      queueMicrotask(() => {
        this.notify(
          [
            {
              target,
              contentRect: box,
              borderBoxSize: [],
              contentBoxSize: [],
              devicePixelContentBoxSize: [],
            },
          ],
          this,
        );
      });
    }

    unobserve(target: Element): void {
      target.hasAttribute("data-message-row");
    }

    disconnect(): void {}
  }

  globalThis.ResizeObserver = Observer;
  Object.defineProperty(HTMLElement.prototype, "offsetParent", {
    configurable: true,
    get() {
      return this.parentElement;
    },
  });

  return () => {
    globalThis.ResizeObserver = previousObserver;

    if (previousOffsetParent !== undefined) {
      Object.defineProperty(HTMLElement.prototype, "offsetParent", previousOffsetParent);
    }
  };
}

describe("permalink placement", () => {
  let loadNewer: ReturnType<typeof vi.spyOn>;
  let restoreMeasure: (() => void) | null = null;
  let previousScrollTo: PropertyDescriptor | undefined;
  let previousObserver: typeof ResizeObserver;

  beforeEach(() => {
    // jsdom does not move focus inside Tab, so the test focuses on the next line. That work
    // already takes longer than the 100ms window when the runner is busy, so the clock stays put.
    resetReaderFocusForTests(() => 0);
    store.setState(initialState, true);
    releaseCards?.();
    releaseCards = null;
    emitScroll = false;
    scrolls.length = 0;
    frames.clear();
    nextFrame = 1;
    previousScrollTo = Object.getOwnPropertyDescriptor(HTMLElement.prototype, "scrollTo");
    previousObserver = globalThis.ResizeObserver;
    globalThis.ResizeObserver = SilentObserver;
    vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => {
      const id = nextFrame;

      nextFrame += 1;
      frames.set(id, callback);

      return id;
    });
    vi.stubGlobal("cancelAnimationFrame", (id: number) => {
      frames.delete(id);
    });
    HTMLElement.prototype.scrollTo = function scrollTo(
      this: HTMLElement,
      xOrOptions?: ScrollToOptions | number,
      y?: number,
    ) {
      const top =
        xOrOptions !== undefined && isScrollOptions(xOrOptions) ? (xOrOptions.top ?? 0) : (y ?? 0);

      scrolls.push(top);
      this.scrollTop = top;

      if (emitScroll) {
        this.dispatchEvent(new Event("scroll"));
      }
    };

    HTMLElement.prototype.scrollBy = function scrollBy() {};

    loadNewer = vi.spyOn(actions, "loadNewer").mockResolvedValue(undefined);
  });

  afterEach(() => {
    resetReaderFocusForTests();
    releaseCards?.();
    releaseCards = null;
    restoreMeasure?.();
    restoreMeasure = null;

    if (previousScrollTo === undefined) {
      Reflect.deleteProperty(HTMLElement.prototype, "scrollTo");
    } else {
      Object.defineProperty(HTMLElement.prototype, "scrollTo", previousScrollTo);
    }

    Reflect.deleteProperty(HTMLElement.prototype, "scrollBy");

    globalThis.ResizeObserver = previousObserver;

    vi.unstubAllGlobals();
    vi.restoreAllMocks();
    openPane(null);
    store.setState(initialState, true);
  });

  it("pages on after a deleted permalink is scrolled to the present and reconnects", async () => {
    // Gone before the list can centre it (deleted while cards were still loading).
    install([1, 2, 4], { after: 4 });
    await renderTimeline(FOCUS);

    // The reader scrolls on to the present. The window then reaches it.
    await act(async () => {
      patchTimeline({ after: null });
    });
    loadNewer.mockClear();

    // A reconnect leaves more than a page beyond the window, with the reader still at its end.
    await act(async () => {
      patchTimeline({ after: 4 });
    });
    expect(loadNewer).toHaveBeenCalledWith(ROOM);
  });

  it("releases newer paging when the permalink list never measures", async () => {
    install([1, 2, FOCUS, 4], { after: 4 });
    await renderTimeline(FOCUS);
    expect(loadNewer).not.toHaveBeenCalled();

    for (let frame = 1; frame < ANCHOR_ATTEMPTS; frame += 1) {
      await flushFrame();
      expect(loadNewer).not.toHaveBeenCalled();
    }

    await flushFrame();
    expect(loadNewer).toHaveBeenCalledTimes(1);
    expect(loadNewer).toHaveBeenCalledWith(ROOM);
  });

  it("recentres a permalink when the around page replaces the cached window", async () => {
    emitScroll = true;
    restoreMeasure = measureList();
    install([1, 2, FOCUS, 4, 5], { after: 5, generation: 1 });
    await renderTimeline(FOCUS);

    const centred = document.querySelector(`[data-message-id="${FOCUS}"]`);

    expect(centred).not.toBeNull();
    expect(scrolls.length).toBeGreaterThan(0);
    scrolls.length = 0;

    // The page around the target replaces the window the cache already held.
    const ids = [10, 11, 12, FOCUS];

    await act(async () => {
      store.setState({
        messages: {
          ...store.getState().messages,
          ...Object.fromEntries(ids.map((id) => [id, messageFixture(id, ROOM)])),
        },
      });
      patchTimeline({ ids, before: 9, after: 13, generation: 2 });
    });

    const recentred = document.querySelector(`[data-message-id="${FOCUS}"]`);

    expect(recentred).not.toBeNull();
    expect(scrolls.length).toBeGreaterThan(0);
    expect(rowIndex(FOCUS)).toBeGreaterThan(0);
  });

  it("does not yank a reader back when a delayed around replacement lands", async () => {
    emitScroll = true;
    restoreMeasure = measureList();
    install([1, 2, FOCUS, 4, 5], { after: 5, generation: 1 });
    await renderTimeline(FOCUS);
    expect(document.querySelector(`[data-message-id="${FOCUS}"]`)).not.toBeNull();

    for (let frame = 0; frame < ANCHOR_ATTEMPTS && frames.size > 0; frame += 1) {
      await flushFrame();
    }

    const list = document.querySelector<HTMLElement>("[data-message-list]");

    expect(list).not.toBeNull();
    scrolls.length = 0;

    // Above the centred row, and short of the end, so following the present cannot explain a jump.
    // A touch is reader input. A later measurement's scroll event is not.
    const away = 48;

    await act(async () => {
      if (list === null) {
        return;
      }

      list.scrollTop = away;
      const start = new TouchEvent("touchstart", { bubbles: true, cancelable: true });

      Object.defineProperty(start, "touches", { value: [{ identifier: 1, clientY: 180 }] });
      list.dispatchEvent(start);
      const move = new TouchEvent("touchmove", { bubbles: true, cancelable: true });

      Object.defineProperty(move, "touches", { value: [{ identifier: 1, clientY: 120 }] });
      list.dispatchEvent(move);
    });

    const held = list?.scrollTop ?? away;
    const ids = [10, 11, 12, FOCUS];

    await act(async () => {
      store.setState({
        messages: {
          ...store.getState().messages,
          ...Object.fromEntries(ids.map((id) => [id, messageFixture(id, ROOM)])),
        },
      });
      patchTimeline({ ids, before: 9, after: 13, generation: 2 });
    });

    expect(scrolls).toEqual([]);
    expect(list?.scrollTop).toBe(held);
  });

  it("releases newer paging when deletion empties the window before placement", async () => {
    install([1, 2, FOCUS, 4], { before: 1, after: 4 });
    await renderTimeline(FOCUS);
    expect(loadNewer).not.toHaveBeenCalled();

    // Every loaded row is deleted before the list can place, and both cursors stay.
    await act(async () => {
      const next = [1, 2, FOCUS, 4].reduce(
        (state, id) => removeMessage(state, id, ROOM, null, Date.now()),
        store.getState(),
      );

      store.setState(next);
    });

    expect(store.getState().timelines[ROOM]?.before).not.toBeNull();
    expect(store.getState().timelines[ROOM]?.after).not.toBeNull();
    expect(store.getState().timelines[ROOM]?.ids).toEqual([]);
    expect(loadNewer).toHaveBeenCalledWith(ROOM);
  });

  it("recentres when a later measurement scrolls without reader input", async () => {
    emitScroll = true;
    restoreMeasure = measureList();
    install([1, 2, FOCUS, 4, 5], { after: 5, generation: 1 });
    await renderTimeline(FOCUS);
    expect(document.querySelector(`[data-message-id="${FOCUS}"]`)).not.toBeNull();

    // The one-frame placing guard has cleared. Virtua then scrolls again while measuring.
    for (let frame = 0; frame < 3; frame += 1) {
      await flushFrame();
    }

    const list = document.querySelector<HTMLElement>("[data-message-list]");

    expect(list).not.toBeNull();
    scrolls.length = 0;

    await act(async () => {
      if (list === null) {
        return;
      }

      list.scrollTop += 16;
      list.dispatchEvent(new Event("scroll"));
    });

    const ids = [10, 11, 12, FOCUS];

    await act(async () => {
      store.setState({
        messages: {
          ...store.getState().messages,
          ...Object.fromEntries(ids.map((id) => [id, messageFixture(id, ROOM)])),
        },
      });
      patchTimeline({ ids, before: 9, after: 13, generation: 2 });
    });

    expect(scrolls.length).toBeGreaterThan(0);
    expect(document.querySelector(`[data-message-id="${FOCUS}"]`)).not.toBeNull();
    expect(rowIndex(FOCUS)).toBeGreaterThan(0);
  });

  it("shows the loaded messages and pages newer when cards never settle", async () => {
    let now = 0;

    vi.spyOn(performance, "now").mockImplementation(() => now);
    releaseCards = holdCardsChunk();
    // Short rows, so the released list is within a page of the newer edge.
    restoreMeasure = measureList(24, 24);
    install([1, 2, FOCUS, 4], { after: 4 });
    store.setState({
      messages: {
        ...store.getState().messages,
        [FOCUS]: messageFixture(FOCUS, ROOM, { poll: heldPoll(FOCUS) }),
      },
    });
    await renderTimeline(FOCUS);

    const shell = document.querySelector(".t-skel");

    expect(shell).not.toBeNull();
    expect(shell?.classList.contains("is-revealed")).toBe(false);
    expect(loadNewer).not.toHaveBeenCalled();

    now = PLACEMENT_WAIT_MS - 1;
    await flushFrame();
    expect(shell?.classList.contains("is-revealed")).toBe(false);
    expect(loadNewer).not.toHaveBeenCalled();

    now = PLACEMENT_WAIT_MS;
    await flushFrame();
    const content = document.querySelector(".t-skel-content");

    expect(shell?.classList.contains("is-revealed")).toBe(true);
    expect(content?.hasAttribute("inert")).toBe(false);
    expect(content?.textContent).toContain("Message 1");
    expect(loadNewer).toHaveBeenCalledWith(ROOM);
  });

  it("does not yank after a wheel when a delayed around replacement lands", async () => {
    emitScroll = true;
    restoreMeasure = measureList();
    install([1, 2, FOCUS, 4, 5], { after: 5, generation: 1 });
    await renderTimeline(FOCUS);
    expect(document.querySelector(`[data-message-id="${FOCUS}"]`)).not.toBeNull();

    for (let frame = 0; frame < ANCHOR_ATTEMPTS && frames.size > 0; frame += 1) {
      await flushFrame();
    }

    const list = document.querySelector<HTMLElement>("[data-message-list]");

    expect(list).not.toBeNull();

    await act(async () => {
      list?.dispatchEvent(
        new WheelEvent("wheel", { deltaY: -48, bubbles: true, cancelable: true }),
      );
    });

    const held = list?.scrollTop ?? 0;

    scrolls.length = 0;

    const ids = [10, 11, 12, FOCUS];

    await act(async () => {
      store.setState({
        messages: {
          ...store.getState().messages,
          ...Object.fromEntries(ids.map((id) => [id, messageFixture(id, ROOM)])),
        },
      });
      patchTimeline({ ids, before: 9, after: 13, generation: 2 });
    });

    expect(scrolls).toEqual([]);
    expect(list?.scrollTop).toBe(held);
  });

  it("does not yank after Tab to an offscreen buffered link when a delayed around replacement lands", async () => {
    const row = 80;
    const box = 240;
    const focus = 15;
    const linkId = 18;
    const ids = Array.from({ length: 30 }, (_, index) => index + 1);

    emitScroll = true;
    restoreMeasure = measureList(row, box);
    install(ids, { before: 1, after: 99, generation: 1 });
    store.setState({
      messages: {
        ...store.getState().messages,
        [linkId]: messageFixture(linkId, ROOM, {
          bodyHtml: '<p><a href="/offscreen">Offscreen note</a></p>',
        }),
      },
    });
    await renderTimeline(focus);
    expect(document.querySelector(`[data-message-id="${focus}"]`)).not.toBeNull();
    await settlePlacement();

    const list = document.querySelector<HTMLElement>("[data-message-list]");
    const link = document.querySelector<HTMLAnchorElement>('a[href="/offscreen"]');
    const linkRow = link?.closest<HTMLElement>("[data-message-row]") ?? null;
    expect(list).not.toBeNull();
    expect(link).not.toBeNull();
    expect(linkRow).not.toBeNull();

    if (list === null || link === null || linkRow === null) {
      return;
    }

    const view = listViewportHeight(list, box);
    const parked = rowTop(linkRow);

    // Mounted in Virtua's buffer, below the centred permalink.
    expect(parked).toBeGreaterThanOrEqual(list.scrollTop + view);

    // jsdom does not move focus on Tab. The key, then the focus, is what the browser does,
    // including the scroll that brings the newly focused control into view.
    await act(async () => {
      document.dispatchEvent(new KeyboardEvent("keydown", { key: "Tab", bubbles: true }));
      link.focus();
      list.scrollTop = parked;
    });

    expect(document.activeElement).toBe(link);
    expect(rowTop(linkRow)).toBeLessThan(list.scrollTop + view);
    scrolls.length = 0;

    await act(async () => {
      patchTimeline({ generation: 2 });
    });

    expect(scrolls).toEqual([]);
    expect(list.scrollTop).toBe(parked);
    expect(document.activeElement).toBe(link);
    expect(link.isConnected).toBe(true);
    expect(rowTop(link.closest("[data-message-row]") ?? linkRow)).toBeLessThan(
      list.scrollTop + view,
    );
  });

  it("still recentres after a dialog returns focus when a delayed around replacement lands", async () => {
    emitScroll = true;
    restoreMeasure = measureList();
    install([1, 2, FOCUS, 4, 5], { after: 5, generation: 1 });
    await renderTimelineDialog(FOCUS);
    expect(document.querySelector(`[data-message-id="${FOCUS}"]`)).not.toBeNull();
    await settlePlacement();

    const row = document.querySelector<HTMLElement>(
      `[data-message-row][data-message-id="${FOCUS}"]`,
    );

    expect(row).not.toBeNull();

    // The permalink placed this row. Opening the dialog from it is not reader input.
    await act(async () => {
      row?.focus();
      setDialogOpen(true);
    });

    const dialog = document.querySelector("dialog");

    expect(dialog).not.toBeNull();
    expect(document.activeElement).not.toBe(row);

    await act(async () => {
      dialog?.dispatchEvent(
        new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }),
      );
      await new Promise((resolve) => {
        setTimeout(resolve, 80);
      });
    });

    expect(document.querySelector("dialog")).toBeNull();
    expect(document.activeElement).toBe(row);
    scrolls.length = 0;

    const ids = [10, 11, 12, FOCUS];

    await act(async () => {
      replaceAround(ids);
    });

    expect(scrolls.length).toBeGreaterThan(0);
    expect(document.querySelector(`[data-message-id="${FOCUS}"]`)).not.toBeNull();
    expect(rowIndex(FOCUS)).toBeGreaterThan(0);
  });

  it("still recentres after Escape returns focus from the right pane", async () => {
    emitScroll = true;
    restoreMeasure = measureList();
    const restoreMedia = installMatchMedia();

    try {
      vi.spyOn(panes, "members").mockResolvedValue({ members: [], users: [] });
      install([1, 2, FOCUS, 4, 5], { after: 5, generation: 1 });
      await renderTimelinePane(FOCUS);
      expect(document.querySelector(`[data-message-id="${FOCUS}"]`)).not.toBeNull();
      await settlePlacement();

      const row = document.querySelector<HTMLElement>(
        `[data-message-row][data-message-id="${FOCUS}"]`,
      );

      expect(row).not.toBeNull();

      if (row === null) {
        return;
      }

      // The permalink left this row focused. Opening the sheet from it is not reader input.
      await act(async () => {
        row.focus();
        openPane("members");
      });

      for (let frame = 0; frame < 5 && document.activeElement === row; frame += 1) {
        await flushFrame();
      }

      const pane = document.querySelector(".right-pane");

      expect(pane).not.toBeNull();
      expect(document.activeElement).not.toBe(row);
      expect(pane?.contains(document.activeElement ?? null)).toBe(true);

      await act(async () => {
        document.activeElement?.dispatchEvent(
          new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }),
        );
      });

      expect(document.activeElement).toBe(row);
      scrolls.length = 0;

      await act(async () => {
        replaceAround([10, 11, 12, FOCUS]);
      });

      expect(scrolls.length).toBeGreaterThan(0);
      expect(document.querySelector(`[data-message-id="${FOCUS}"]`)).not.toBeNull();
      expect(rowIndex(FOCUS)).toBeGreaterThan(0);
    } finally {
      restoreMedia();
    }
  });

  it("still recentres after Tab in the right pane then a close pointer within 100ms", async () => {
    emitScroll = true;
    restoreMeasure = measureList();
    const restoreMedia = installMatchMedia();

    try {
      vi.spyOn(panes, "members").mockResolvedValue({ members: [], users: [] });
      install([1, 2, FOCUS, 4, 5], { after: 5, generation: 1 });
      await renderTimelinePane(FOCUS);
      expect(document.querySelector(`[data-message-id="${FOCUS}"]`)).not.toBeNull();
      await settlePlacement();

      const row = document.querySelector<HTMLElement>(
        `[data-message-row][data-message-id="${FOCUS}"]`,
      );

      expect(row).not.toBeNull();

      if (row === null) {
        return;
      }

      // The permalink left this row focused. Opening the sheet from it is not reader input.
      await act(async () => {
        row.focus();
        openPane("members");
      });

      for (let frame = 0; frame < 5 && document.activeElement === row; frame += 1) {
        await flushFrame();
      }

      const pane = document.querySelector(".right-pane");
      const close = pane?.querySelector<HTMLButtonElement>('button[aria-label="Close"]');

      expect(pane).not.toBeNull();
      expect(close).not.toBeNull();
      expect(document.activeElement).not.toBe(row);
      expect(pane?.contains(document.activeElement ?? null)).toBe(true);

      // Tab inside the sheet, then Close in the same window. The click hands focus back
      // to the timeline row. That hand-back is the pane's, so the late page still centres.
      await act(async () => {
        document.activeElement?.dispatchEvent(
          new KeyboardEvent("keydown", { key: "Tab", bubbles: true }),
        );
        close?.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
        close?.click();
      });

      expect(document.activeElement).toBe(row);
      scrolls.length = 0;

      await act(async () => {
        replaceAround([10, 11, 12, FOCUS]);
      });

      expect(scrolls.length).toBeGreaterThan(0);
      expect(document.querySelector(`[data-message-id="${FOCUS}"]`)).not.toBeNull();
      expect(rowIndex(FOCUS)).toBeGreaterThan(0);
    } finally {
      restoreMedia();
    }
  });

  it("still recentres after an emoji picker autofocuses inside the timeline", async () => {
    emitScroll = true;
    restoreMeasure = measureList();
    install([1, 2, FOCUS, 4, 5], { after: 5, generation: 1 });
    store.setState({ boot: VIEWER });
    await renderTimeline(FOCUS);
    expect(document.querySelector(`[data-message-id="${FOCUS}"]`)).not.toBeNull();
    await settlePlacement();

    const row = document.querySelector<HTMLElement>(
      `[data-message-row][data-message-id="${FOCUS}"]`,
    );

    const list = document.querySelector<HTMLElement>("[data-message-list]");

    expect(row).not.toBeNull();
    expect(list).not.toBeNull();

    if (row === null || list === null) {
      return;
    }

    await act(async () => {
      row.focus();
      row.dispatchEvent(new KeyboardEvent("keydown", { key: "r", bubbles: true }));
    });

    const picker = document.querySelector("[aria-label='Add a reaction']");

    expect(picker).not.toBeNull();
    expect(document.activeElement).not.toBe(row);
    expect(list.contains(document.activeElement)).toBe(true);
    scrolls.length = 0;

    await act(async () => {
      replaceAround([10, 11, 12, FOCUS]);
    });

    expect(scrolls.length).toBeGreaterThan(0);
    expect(document.querySelector(`[data-message-id="${FOCUS}"]`)).not.toBeNull();
    expect(rowIndex(FOCUS)).toBeGreaterThan(0);
  });
});

function heldPoll(messageId: number): Poll {
  return {
    id: 40,
    messageId,
    asOf: "2026-10-06T00:00:00.000Z",
    multiple: false,
    anonymous: false,
    closesAt: null,
    closedAt: null,
    closed: false,
    totalVotes: 0,
    options: [{ id: 401, label: "Tea", votes: 0, voterIds: [] }],
  };
}
