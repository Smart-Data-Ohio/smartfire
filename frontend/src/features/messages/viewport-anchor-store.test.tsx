import {
  createMemoryHistory,
  createRootRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, cleanup, fireEvent, render } from "@testing-library/react";
import { createRef, type RefObject, useImperativeHandle, useRef } from "react";
import { VList, type VListHandle } from "virtua";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { messageFixture, pageFixture } from "../../api/testing.ts";
import type { MessageDTO } from "../../store/model.ts";
import { initialState } from "../../store/state.ts";
import { mutations, store, useStore } from "../../store/store.ts";
import { Timeline } from "../room/timeline.tsx";
import { type TimelineItem, timelineItems } from "../room/timeline-items.ts";
import { useViewportAnchor } from "./viewport-anchor.ts";

type AnchorApi = ReturnType<typeof useViewportAnchor>;

const ROOM = 12;

const THREAD = 88;

const VIEWPORT_HEIGHT = 300;

class MeasuringObserver implements ResizeObserver {
  static instances: MeasuringObserver[] = [];
  readonly observed = new Set<Element>();
  readonly callback: ResizeObserverCallback;

  constructor(callback: ResizeObserverCallback) {
    this.callback = callback;
    MeasuringObserver.instances.push(this);
  }

  observe(target: Element): void {
    this.observed.add(target);
  }

  unobserve(target: Element): void {
    this.observed.delete(target);
  }

  disconnect(): void {
    this.observed.clear();
  }

  static deliver(measurements: readonly (readonly [Element, number])[]): void {
    for (const observer of MeasuringObserver.instances) {
      const entries = measurements.flatMap(([target, height]): ResizeObserverEntry[] =>
        observer.observed.has(target)
          ? [
              {
                target,
                contentRect: new DOMRectReadOnly(0, 0, 600, height),
                borderBoxSize: [],
                contentBoxSize: [],
                devicePixelContentBoxSize: [],
              },
            ]
          : [],
      );

      if (entries.length > 0) observer.callback(entries, observer);
    }
  }
}

function messageHeight(message: MessageDTO): number {
  if (message.bodyHtml.includes("Destination growth")) return 600;

  if (message.bodyHtml.includes("Tall destination")) return 500;

  if (message.bodyHtml.includes("Jump growth")) return 260;

  return message.bodyHtml.includes("Grown message") ? 500 : 200;
}

/** Store reconciliation and Virtua are real; jsdom needs committed layout measurements. */
function StoreTimeline({
  apiRef,
  thread = false,
  popupId,
  navigationRef,
  measuredLayout = false,
}: {
  readonly apiRef: RefObject<AnchorApi | null>;
  readonly thread?: boolean;
  readonly popupId?: number;
  readonly navigationRef?: RefObject<VListHandle | null>;
  readonly measuredLayout?: boolean;
}) {
  const state = useStore((state) => state);
  const timeline = thread ? state.threadTimelines[THREAD] : state.timelines[ROOM];
  const parent = thread ? (state.messages[1] ?? null) : null;
  const containerRef = useRef<HTMLDivElement | null>(null);
  const listRef = useRef<VListHandle | null>(null);

  if (!timeline) throw new Error("The store timeline has not loaded");

  const items = timelineItems({ timeline, messages: state.messages, pending: [], now: 0 });

  const heights = items.map((item) =>
    item.kind === "message"
      ? messageHeight(item.message)
      : item.kind === "intro" && parent
        ? messageHeight(parent)
        : 0,
  );

  const offsets = heights.map((_, index) => heights.slice(0, index).reduce((a, b) => a + b, 0));
  const total = heights.reduce((a, b) => a + b, 0);

  const anchor = useViewportAnchor({
    containerRef,
    listRef,
    items,
    parentId: parent?.id ?? null,
    placement: thread ? "thread:88" : "room:12",
    placed: true,
    cardsLoaded: true,
  });

  useImperativeHandle(apiRef, () => anchor);
  useImperativeHandle(navigationRef, () => {
    if (!listRef.current) throw new Error("Virtua has not mounted");

    return listRef.current;
  });
  const viewport = () => containerRef.current?.querySelector<HTMLElement>('[role="log"]');

  const renderItem = (item: TimelineItem, index: number) => {
    const message = item.kind === "message" ? item.message : null;
    const id = message?.id ?? (item.kind === "intro" ? parent?.id : undefined);

    return (
      <div
        key={item.key}
        data-item-key={item.key}
        data-layout-height={heights[index]}
        className={item.kind === "intro" && thread ? "thread-parent" : undefined}
        ref={(row) => {
          if (!row) return;

          const bounds = () =>
            new DOMRect(
              0,
              (offsets[index] ?? 0) - (viewport()?.scrollTop ?? 0),
              600,
              heights[index],
            );

          row.getBoundingClientRect = bounds;

          if (row.parentElement) {
            row.parentElement.getBoundingClientRect = bounds;
            Object.defineProperty(row.parentElement, "offsetParent", {
              configurable: true,
              get: viewport,
            });
          }

          const messageRow = row.querySelector<HTMLElement>("[data-message-row]");

          if (messageRow) messageRow.getBoundingClientRect = bounds;
        }}
      >
        {id === undefined ? (
          item.kind === "intro" && thread ? (
            "The original message was deleted."
          ) : null
        ) : (
          <article data-message-row data-message-id={id}>
            Message {id}
            {popupId === id ? <span className="message-popup-anchor" /> : null}
          </article>
        )}
      </div>
    );
  };

  return (
    <div
      ref={(container) => {
        containerRef.current = container;
        const element = viewport();

        if (!element) return;

        Object.defineProperties(element, {
          clientHeight: { configurable: true, value: VIEWPORT_HEIGHT },
          scrollHeight: {
            configurable: true,
            get: () =>
              measuredLayout && element.firstElementChild instanceof HTMLElement
                ? Number.parseFloat(element.firstElementChild.style.height)
                : total,
          },
        });
        // Native clamping happens at the DOM commit, after the anchor's render snapshot.
        element.scrollTop = Math.max(
          0,
          Math.min(element.scrollTop, element.scrollHeight - VIEWPORT_HEIGHT),
        );
        element.getBoundingClientRect = () => new DOMRect(0, 0, 600, VIEWPORT_HEIGHT);
        element.scrollBy = (
          ...args: [options?: ScrollToOptions | undefined] | [x: number, y: number]
        ) => {
          element.scrollTop += args.length === 2 ? args[1] : (args[0]?.top ?? 0);
        };
      }}
    >
      <VList
        ref={listRef}
        role="log"
        itemSize={200}
        bufferSize={measuredLayout ? 0 : 2000}
        data={items}
        onScroll={() => anchor.capture()}
        onScrollEnd={() => anchor.settle()}
      >
        {renderItem}
      </VList>
    </div>
  );
}

function viewport(): HTMLElement {
  const element = document.querySelector<HTMLElement>('[role="log"]');

  if (!element) throw new Error("The message list was not drawn");

  return element;
}

async function measure(): Promise<void> {
  await act(async () => {
    const element = viewport();
    const content = element.firstElementChild;

    if (!content) throw new Error("The message list has no content");

    MeasuringObserver.deliver([
      [element, VIEWPORT_HEIGHT],
      [content, element.scrollHeight],
      ...Array.from(content.children, (wrapper): readonly [Element, number] => [
        wrapper,
        Number(wrapper.firstElementChild?.getAttribute("data-layout-height")),
      ]),
    ]);
  });
}

/** Animate the native boundary; the target and premeasurement remain Virtua's. */
function animateScroll(element: HTMLElement) {
  let animation = 0;

  const scrollTo = vi.fn(
    (...args: [options?: ScrollToOptions | undefined] | [x: number, y: number]) => {
      const options: ScrollToOptions =
        args.length === 2 ? { left: args[0], top: args[1] } : (args[0] ?? {});

      cancelAnimationFrame(animation);
      const start = element.scrollTop;
      const destination = options.top ?? start;
      let frame = 0;

      const advance = () => {
        frame += 1;
        element.scrollTop = Math.max(
          0,
          Math.min(
            start + (destination - start) * (options.behavior === "smooth" ? frame / 4 : 1),
            element.scrollHeight - element.clientHeight,
          ),
        );
        fireEvent.scroll(element);

        if (options.behavior === "smooth" && frame < 4) animation = requestAnimationFrame(advance);
        else fireEvent(element, new Event("scrollend"));
      };

      if (options.behavior === "smooth") animation = requestAnimationFrame(advance);
      else advance();
    },
  );

  element.scrollTo = scrollTo;

  return scrollTo;
}

/** jsdom geometry for the production component and its real rows. */
async function measureTimeline() {
  const element = viewport();
  const content = element.firstElementChild;

  if (!(content instanceof HTMLElement)) throw new Error("Timeline has no content");

  Object.defineProperties(element, {
    clientHeight: { configurable: true, value: VIEWPORT_HEIGHT },
    scrollHeight: { configurable: true, get: () => Number.parseFloat(content.style.height) },
  });
  element.getBoundingClientRect = () => new DOMRect(0, 0, 600, VIEWPORT_HEIGHT);
  element.scrollBy = (
    ...args: [options?: ScrollToOptions | undefined] | [x: number, y: number]
  ) => {
    element.scrollTop += args.length === 2 ? args[1] : (args[0]?.top ?? 0);
  };

  for (const wrapper of content.children) {
    if (!(wrapper instanceof HTMLElement)) continue;

    Object.defineProperty(wrapper, "offsetParent", { configurable: true, get: () => element });

    const row = wrapper.firstElementChild;

    if (row)
      row.setAttribute("data-layout-height", row.matches("[data-message-row]") ? "200" : "0");

    const bounds = () =>
      new DOMRect(
        0,
        Number.parseFloat(wrapper.style.top) - element.scrollTop,
        600,
        Number(wrapper.firstElementChild?.getAttribute("data-layout-height")),
      );

    wrapper.getBoundingClientRect = bounds;

    if (wrapper.firstElementChild instanceof HTMLElement)
      wrapper.firstElementChild.getBoundingClientRect = bounds;
  }

  await measure();
}

function remove(message: MessageDTO): void {
  mutations.applyEvents(
    [
      {
        seq: 1,
        topic: `room:${ROOM}`,
        type: "message.removed",
        data: { id: message.id, roomId: ROOM, threadId: message.threadId },
      },
    ],
    0,
  );
}

describe("store-backed deletion with real Virtua", () => {
  beforeEach(() => {
    store.setState(initialState, true);
    MeasuringObserver.instances = [];
    vi.stubGlobal("ResizeObserver", MeasuringObserver);
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
    store.setState(initialState, true);
  });

  it("production Timeline preserves forward find through last-row deletion before settlement", async () => {
    vi.useFakeTimers({ toFake: ["requestAnimationFrame", "cancelAnimationFrame"] });

    try {
      mutations.applyPage(
        ROOM,
        pageFixture([1, 2, 3, 4, 5].map((id) => messageFixture(id, ROOM))),
        "replace",
      );

      const rootRoute = createRootRoute({
        component: () => <Timeline roomId={ROOM} focusMessageId={null} />,
      });

      const router = createRouter({
        routeTree: rootRoute,
        history: createMemoryHistory({ initialEntries: ["/"] }),
      });

      const view = render(<RouterProvider router={router} />);

      await act(() => router.load());

      for (
        let frame = 0;
        frame < 10 && viewport().dataset.placementSettled !== "true";
        frame += 1
      ) {
        await measureTimeline();
        await act(async () => vi.advanceTimersToNextFrame());
        fireEvent.scroll(viewport());
      }

      expect(viewport().dataset.placementSettled).toBe("true");
      expect(viewport().scrollTop).toBe(700);
      const element = viewport();
      const scrollTo = animateScroll(element);

      // Start from an older reader, then let production Jump to present issue smooth motion.
      element.scrollTop = 100;
      fireEvent.scroll(element);
      fireEvent(element, new Event("scrollend"));
      await act(async () => fireEvent.click(view.getByRole("button", { name: "Jump to present" })));
      expect(scrollTo).toHaveBeenLastCalledWith({ top: 700, behavior: "smooth" });
      await act(async () => vi.advanceTimersToNextFrame());
      await measureTimeline();
      // Find-in-page advances inside the issued range and stops native animation.
      // Deliberately leave its scroll/scrollend undelivered until after the deletion commit.
      element.scrollTop += 80;
      const stopped = element.scrollTop;

      expect(stopped).toBeLessThan(500);
      await act(async () => remove(messageFixture(5, ROOM)));
      await measureTimeline();
      expect(store.getState().timelines[ROOM]?.ids).toEqual([1, 2, 3, 4]);
      expect(element.scrollTop).toBe(stopped);
      expect(scrollTo).toHaveBeenCalledTimes(1);
      fireEvent.scroll(element);
      fireEvent(element, new Event("scrollend"));
      await act(async () => mutations.receiveMessage(messageFixture(6, ROOM)));
      await measureTimeline();
      expect(element.scrollTop).toBe(stopped);
      expect(scrollTo).toHaveBeenCalledTimes(1);
    } finally {
      vi.useRealTimers();
    }
  });

  it("lets Virtua premeasure and smoothly reach a taller live end", async () => {
    vi.useFakeTimers({ toFake: ["requestAnimationFrame", "cancelAnimationFrame"] });

    try {
      const messages = Array.from({ length: 20 }, (_, index) =>
        messageFixture(index + 1, ROOM, {
          bodyHtml: index === 19 ? "<p>Tall destination</p>" : "<p>A preceding message</p>",
        }),
      );

      mutations.applyPage(ROOM, pageFixture(messages), "replace");
      const apiRef = createRef<AnchorApi>();
      const navigationRef = createRef<VListHandle>();

      render(<StoreTimeline apiRef={apiRef} navigationRef={navigationRef} measuredLayout />);
      await measure();
      const element = viewport();
      const scrollTo = animateScroll(element);
      await act(async () => navigationRef.current?.scrollTo(100));
      await measure();
      expect(element.querySelector('[data-message-id="20"]')).toBeNull();
      expect(element.scrollHeight - element.clientHeight).toBe(3700);
      await act(async () => {
        apiRef.current?.followEnd();
        navigationRef.current?.scrollToIndex(
          (store.getState().timelines[ROOM]?.ids.length ?? 0) + 1,
          { align: "end", smooth: true },
        );
      });
      // The destination enters Virtua's premeasurement range before native motion starts.
      expect(element.querySelector('[data-message-id="20"]')).not.toBeNull();
      await measure();
      expect(scrollTo).toHaveBeenLastCalledWith({ top: 4000, behavior: "smooth" });
      expect(element.scrollTop).toBe(100);
      await act(async () => vi.advanceTimersToNextFrame());
      await measure();
      expect(element.scrollTop).toBeGreaterThan(100);
      expect(element.scrollTop).toBeLessThan(3700);
      act(() =>
        mutations.updateMessage(
          messageFixture(20, ROOM, {
            bodyHtml: "<p>Destination growth</p>",
            updatedAt: "2026-10-06T00:01:00.000Z",
          }),
        ),
      );
      await measure();
      expect(element.scrollHeight - element.clientHeight).toBe(4100);

      for (let frame = 0; frame < 3; frame += 1) {
        await act(async () => vi.advanceTimersToNextFrame());
        await measure();
      }

      expect(element.scrollTop).toBe(4100);
      expect(apiRef.current?.canFollow()).toBe(true);
      act(() => mutations.receiveMessage(messageFixture(21, ROOM)));
      await measure();
      expect(element.scrollTop).toBe(4300);
      expect(apiRef.current?.canFollow()).toBe(true);
    } finally {
      vi.useRealTimers();
    }
  });

  it("keeps an unfinished issued jump through growth and a store removal", async () => {
    mutations.applyPage(
      ROOM,
      pageFixture([1, 2, 3, 4, 5].map((id) => messageFixture(id, ROOM))),
      "replace",
    );
    const apiRef = createRef<AnchorApi>();

    render(<StoreTimeline apiRef={apiRef} />);
    await measure();
    viewport().scrollTop = 100;
    act(() => apiRef.current?.followEnd());
    viewport().scrollTop = 300;
    act(() =>
      mutations.updateMessage(
        messageFixture(5, ROOM, {
          bodyHtml: "<p>Jump growth</p>",
          updatedAt: "2026-10-06T00:01:00.000Z",
        }),
      ),
    );
    await measure();
    expect(viewport().scrollHeight).toBe(1060);
    expect(viewport().scrollTop).toBe(300);
    act(() => remove(messageFixture(5, ROOM)));
    expect(store.getState().timelines[ROOM]?.ids).toEqual([1, 2, 3, 4]);
    await measure();
    expect(viewport().scrollTop).toBe(300);
    viewport().scrollTop = 500;
    fireEvent.scroll(viewport());
    act(() => apiRef.current?.settle());
    expect(apiRef.current?.canFollow()).toBe(true);
    act(() => mutations.receiveMessage(messageFixture(6, ROOM)));
    await measure();
    expect(viewport().scrollTop).toBe(700);
    expect(apiRef.current?.canFollow()).toBe(true);
  });

  it("cannot repin an eventless find to the paused end after deleting a non-witness", async () => {
    mutations.applyPage(
      ROOM,
      pageFixture([1, 2, 3].map((id) => messageFixture(id, ROOM))),
      "replace",
    );
    const apiRef = createRef<AnchorApi>();
    const view = render(<StoreTimeline apiRef={apiRef} />);

    await measure();
    viewport().scrollTop = 300;
    act(() => apiRef.current?.followEnd());
    await measure();
    expect(viewport().scrollTop).toBe(300);
    expect(document.querySelector('[data-message-id="2"]')?.getBoundingClientRect().top).toBe(-100);
    view.rerender(<StoreTimeline apiRef={apiRef} popupId={3} />);
    act(() =>
      mutations.updateMessage(
        messageFixture(3, ROOM, {
          bodyHtml: "<p>Grown message</p>",
          updatedAt: "2026-10-06T00:01:00.000Z",
        }),
      ),
    );
    await measure();
    expect(viewport().scrollHeight).toBe(900);
    expect(viewport().scrollTop).toBe(300);
    viewport().scrollTop = 600;
    act(() => remove(messageFixture(3, ROOM)));
    expect(store.getState().timelines[ROOM]?.ids).toEqual([1, 2]);
    expect(viewport().scrollTop).toBe(100);
    await measure();
    fireEvent.scroll(viewport());
    act(() => apiRef.current?.settle());
    act(() => mutations.receiveMessage(messageFixture(4, ROOM)));
    await measure();
    expect(viewport().scrollTop).toBe(100);
    expect(apiRef.current?.canFollow()).toBe(false);
  });

  it.each(["idle", "paused"])(
    "snapshots eventless movement before deleting the thread parent with its intro key intact (%s)",
    async (pause) => {
      mutations.applyPage(ROOM, pageFixture([messageFixture(1, ROOM)]), "replace");
      mutations.applyThreadPage(
        THREAD,
        pageFixture([2, 3].map((id) => messageFixture(id, ROOM, { threadId: THREAD }))),
        "replace",
      );
      const apiRef = createRef<AnchorApi>();
      const view = render(<StoreTimeline apiRef={apiRef} thread />);

      await measure();
      viewport().scrollTop = 300;
      act(() => apiRef.current?.followEnd());
      await measure();

      if (pause === "paused") view.rerender(<StoreTimeline apiRef={apiRef} thread popupId={3} />);

      viewport().scrollTop = 100;
      act(() => remove(messageFixture(1, ROOM)));
      expect(document.querySelector('[data-item-key="intro"]')?.textContent).toContain("deleted");
      expect(store.getState().threadTimelines[THREAD]?.ids).toEqual([2, 3]);
      expect(viewport().scrollTop).toBe(100);
      await measure();
      view.rerender(<StoreTimeline apiRef={apiRef} thread />);
      await act(async () => undefined);
      fireEvent.scroll(viewport());
      act(() => apiRef.current?.settle());
      act(() => mutations.receiveMessage(messageFixture(4, ROOM, { threadId: THREAD })));
      await measure();
      expect(viewport().scrollTop).toBe(100);
      expect(apiRef.current?.canFollow()).toBe(false);
    },
  );
});
