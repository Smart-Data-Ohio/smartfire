import {
  createMemoryHistory,
  createRootRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { messageFixture } from "../../api/testing.ts";
import type { Timeline as RoomTimeline } from "../../store/model.ts";
import { emptyTimeline, initialState } from "../../store/state.ts";
import { store } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { Timeline } from "./timeline.tsx";
import { timelineItems } from "./timeline-items.ts";

const ROOM = 12;

const FOCUS = 3;

/** Frames the timeline waits for a viewport before it must release newer paging anyway. */
const ANCHOR_ATTEMPTS = 30;

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
function measureList(): () => void {
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
      const height = target.hasAttribute("data-message-row") ? 48 : 360;
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
    store.setState(initialState, true);
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
});
