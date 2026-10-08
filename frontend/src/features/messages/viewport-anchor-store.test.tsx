import { act, cleanup, fireEvent, render } from "@testing-library/react";
import { createRef, type RefObject, useImperativeHandle, useRef } from "react";
import { VList, type VListHandle } from "virtua";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { messageFixture, pageFixture } from "../../api/testing.ts";
import type { MessageDTO } from "../../store/model.ts";
import { initialState } from "../../store/state.ts";
import { mutations, store, useStore } from "../../store/store.ts";
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
  if (message.bodyHtml.includes("Jump growth")) return 260;

  return message.bodyHtml.includes("Grown message") ? 500 : 200;
}

/** Store reconciliation and Virtua are real; jsdom needs committed layout measurements. */
function StoreTimeline({
  apiRef,
  thread = false,
  popupId,
}: {
  readonly apiRef: RefObject<AnchorApi | null>;
  readonly thread?: boolean;
  readonly popupId?: number;
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
          scrollHeight: { configurable: true, value: total },
        });
        // Native clamping happens at the DOM commit, after the anchor's render snapshot.
        element.scrollTop = Math.max(0, Math.min(element.scrollTop, total - VIEWPORT_HEIGHT));
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
        bufferSize={2000}
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
