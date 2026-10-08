import { act, fireEvent, render } from "@testing-library/react";
import { createRef, type RefObject, useImperativeHandle, useRef } from "react";
import type { VListHandle } from "virtua";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { messageFixture } from "../../api/testing.ts";
import type { TimelineItem } from "../room/timeline-items.ts";
import { useViewportAnchor } from "./viewport-anchor.ts";

type AnchorApi = ReturnType<typeof useViewportAnchor>;

/** Deliver only the measurements a browser would send to each observer. */
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

interface Geometry {
  ids: readonly number[];
  readonly heights: Map<number, number>;
}

const VIEWPORT_HEIGHT = 300;

function heightOf(geometry: Geometry, id: number): number {
  return geometry.heights.get(id) ?? 200;
}

function offsetOf(geometry: Geometry, index: number): number {
  return geometry.ids.slice(0, index).reduce((total, id) => total + heightOf(geometry, id), 0);
}

interface HarnessProps {
  readonly apiRef: RefObject<AnchorApi | null>;
  readonly geometry: Geometry;
  readonly drawn?: readonly number[];
  readonly cardsLoaded?: boolean;
  readonly hasCards?: boolean;
  readonly popupId?: number;
  readonly editingId?: number;
  readonly popupPendingId?: number;
  readonly virtual?: typeof import("virtua").VList;
  readonly onSettled?: () => void;
}

/** The virtualiser's geometry is explicit because jsdom does not lay out the rows. */
function Harness({
  apiRef,
  geometry,
  drawn = geometry.ids,
  cardsLoaded = true,
  hasCards = false,
  popupId,
  editingId,
  popupPendingId,
  virtual: VirtualList,
  onSettled,
}: HarnessProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const viewport = () => containerRef.current?.querySelector<HTMLElement>('[role="log"]');

  const listRef = useRef<VListHandle>({
    cache: [[]],
    get scrollOffset() {
      return viewport()?.scrollTop ?? 0;
    },
    get scrollSize() {
      return offsetOf(geometry, geometry.ids.length);
    },
    viewportSize: VIEWPORT_HEIGHT,
    getItemOffset: (index) => offsetOf(geometry, index),
    getItemSize: (index) => heightOf(geometry, geometry.ids[index] ?? -1),
    findItemIndex: () => 0,
    scrollToIndex: () => undefined,
    scrollTo: () => undefined,
    scrollBy: () => undefined,
  });

  const items: TimelineItem[] = geometry.ids.map((id) => ({
    kind: "message",
    key: `c-${id}`,
    message: messageFixture(id, 12),
    groupStart: true,
  }));

  const anchor = useViewportAnchor({
    containerRef,
    listRef,
    items,
    placement: "room:12",
    placed: true,
    cardsLoaded,
  });

  useImperativeHandle(apiRef, () => anchor);

  const rowBounds = (id: number) =>
    new DOMRect(
      0,
      offsetOf(geometry, geometry.ids.indexOf(id)) - (viewport()?.scrollTop ?? 0),
      600,
      heightOf(geometry, id),
    );

  if (VirtualList) {
    return (
      <div
        ref={(container) => {
          containerRef.current = container;
          const element = container?.querySelector<HTMLElement>('[role="log"]');

          if (!element) return;

          Object.defineProperties(element, {
            clientHeight: { configurable: true, value: VIEWPORT_HEIGHT },
            scrollHeight: {
              configurable: true,
              get: () => offsetOf(geometry, geometry.ids.length),
            },
          });
          element.getBoundingClientRect = () => new DOMRect(0, 0, 600, VIEWPORT_HEIGHT);
        }}
      >
        <VirtualList
          ref={listRef}
          role="log"
          itemSize={200}
          onScroll={() => anchor.capture()}
          onScrollEnd={() => {
            onSettled?.();
            anchor.settle();
          }}
        >
          {geometry.ids.map((id) => (
            <article
              key={id}
              data-message-row
              data-message-id={id}
              ref={(row) => {
                if (!row) return;

                row.getBoundingClientRect = () => rowBounds(id);

                if (row.parentElement) {
                  row.parentElement.getBoundingClientRect = () => rowBounds(id);
                  Object.defineProperty(row.parentElement, "offsetParent", {
                    configurable: true,
                    value: viewport(),
                  });
                }
              }}
            >
              Message {id}
            </article>
          ))}
        </VirtualList>
      </div>
    );
  }

  return (
    <div ref={containerRef}>
      <div
        role="log"
        aria-label="Messages"
        tabIndex={-1}
        onScroll={() => anchor.capture()}
        ref={(element) => {
          if (element === null) return;

          Object.defineProperties(element, {
            clientHeight: { configurable: true, value: VIEWPORT_HEIGHT },
            scrollHeight: {
              configurable: true,
              get: () => offsetOf(geometry, geometry.ids.length),
            },
          });
          element.getBoundingClientRect = () => new DOMRect(0, 0, 600, VIEWPORT_HEIGHT);
        }}
      >
        <div>
          {drawn.map((id) => (
            <div
              key={id}
              ref={(element) => {
                if (element) element.getBoundingClientRect = () => rowBounds(id);
              }}
            >
              <article
                data-message-row
                data-message-id={id}
                data-editing={editingId === id ? true : undefined}
                data-popup-pending={popupPendingId === id ? true : undefined}
                tabIndex={-1}
                ref={(element) => {
                  if (element) element.getBoundingClientRect = () => rowBounds(id);
                }}
              >
                <div className="message-body">Message {id}</div>
                <a href="/">Read message {id}</a>
                {hasCards ? <div className="message-cards">Card</div> : null}
                {popupId === id ? <span className="message-popup-anchor" /> : null}
              </article>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

function rowOf(id: number): HTMLElement {
  const row = document.querySelector<HTMLElement>(`[data-message-id="${id}"]`);

  if (row === null) throw new Error(`Message ${id} was not drawn`);

  return row;
}

function viewport(): HTMLElement {
  const element = document.querySelector<HTMLElement>('[role="log"]');

  if (element === null) throw new Error("The message list was not drawn");

  return element;
}

function measureRows(geometry: Geometry): void {
  const list = viewport();
  const content = list.firstElementChild;

  if (content === null) throw new Error("The message list has no content");

  MeasuringObserver.deliver([
    [content, offsetOf(geometry, geometry.ids.length)],
    ...Array.from(content.children, (wrapper): readonly [Element, number] => {
      const id = Number(wrapper.firstElementChild?.getAttribute("data-message-id"));

      return [wrapper, heightOf(geometry, id)];
    }),
  ]);
}

function follow(apiRef: RefObject<AnchorApi | null>, geometry: Geometry): void {
  viewport().scrollTop = offsetOf(geometry, geometry.ids.length) - VIEWPORT_HEIGHT;
  act(() => apiRef.current?.followEnd());
  act(() => measureRows(geometry));
}

describe("useViewportAnchor reader control", () => {
  beforeEach(() => {
    MeasuringObserver.instances = [];
    vi.stubGlobal("ResizeObserver", MeasuringObserver);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("follows mixed hidden and visible growth after partial Virtua compensation", () => {
    const apiRef = createRef<AnchorApi>();
    const geometry: Geometry = { ids: [1, 2, 3], heights: new Map() };

    render(<Harness apiRef={apiRef} geometry={geometry} />);
    follow(apiRef, geometry);
    fireEvent.scroll(viewport());
    geometry.heights.set(1, 300);
    geometry.heights.set(3, 300);
    // Virtua compensates the hidden row before our observer sees both resizes.
    viewport().scrollTop = 400;
    act(() => measureRows(geometry));
    expect(viewport().scrollTop).toBe(500);
    expect(apiRef.current?.canFollow()).toBe(true);
  });

  it("follows visible growth when a hidden row shrinks in the same delivery", () => {
    const apiRef = createRef<AnchorApi>();
    const geometry: Geometry = { ids: [1, 2, 3], heights: new Map() };

    render(<Harness apiRef={apiRef} geometry={geometry} />);
    follow(apiRef, geometry);
    geometry.heights.set(1, 100);
    geometry.heights.set(3, 400);
    // The hidden shrink is compensated, leaving the visible row at its original top.
    viewport().scrollTop = 200;
    act(() => measureRows(geometry));
    expect(viewport().scrollTop).toBe(400);
    expect(apiRef.current?.canFollow()).toBe(true);
  });

  it("find-in-page within hidden compensation cancels paused follow", async () => {
    const apiRef = createRef<AnchorApi>();
    const geometry: Geometry = { ids: [1, 2, 3], heights: new Map() };
    const view = render(<Harness apiRef={apiRef} geometry={geometry} />);

    follow(apiRef, geometry);
    view.rerender(<Harness apiRef={apiRef} geometry={geometry} popupId={3} />);
    geometry.heights.set(1, 400);
    geometry.heights.set(3, 300);
    viewport().scrollTop = 500;
    act(() => measureRows(geometry));
    // Browser movement is backward from compensation, but forward from the last pin.
    viewport().scrollTop = 400;
    fireEvent.scroll(viewport());
    view.rerender(<Harness apiRef={apiRef} geometry={geometry} />);
    await act(async () => undefined);
    expect(viewport().scrollTop).toBe(400);
    expect(apiRef.current?.canFollow()).toBe(false);
  });

  it.each(["menu", "editor"])(
    "restoring focus after an older %s preserves the paused row position",
    async (interaction) => {
      const apiRef = createRef<AnchorApi>();
      const geometry: Geometry = { ids: [1, 2, 3], heights: new Map() };

      const view = render(
        <Harness apiRef={apiRef} geometry={geometry} cardsLoaded={false} hasCards />,
      );

      follow(apiRef, geometry);
      act(() => rowOf(2).focus());
      const top = rowOf(2).getBoundingClientRect().top;

      view.rerender(
        <Harness
          apiRef={apiRef}
          geometry={geometry}
          cardsLoaded={false}
          hasCards
          {...(interaction === "menu" ? { popupId: 2 } : { editingId: 2 })}
        />,
      );
      act(() => rowOf(2).querySelector<HTMLAnchorElement>("a")?.focus());
      geometry.heights.set(1, 300);
      viewport().scrollTop = 350;
      act(() => measureRows(geometry));
      fireEvent.scroll(viewport());
      expect(rowOf(2).getBoundingClientRect().top).toBe(top + 50);
      // Both Esc and closing an editor restore focus before removing the marker.
      act(() => rowOf(2).focus());
      view.rerender(<Harness apiRef={apiRef} geometry={geometry} cardsLoaded hasCards />);
      await act(async () => undefined);
      expect(rowOf(2).getBoundingClientRect().top).toBe(top);
      expect(viewport().scrollTop).toBe(400);
      expect(apiRef.current?.canFollow()).toBe(false);
    },
  );

  it("a forward interruption of jump-to-latest stops short without snapping on growth", () => {
    const apiRef = createRef<AnchorApi>();
    const geometry: Geometry = { ids: [1, 2, 3, 4, 5], heights: new Map() };

    render(<Harness apiRef={apiRef} geometry={geometry} />);
    viewport().scrollTop = 100;
    act(() => apiRef.current?.followEnd());
    act(() => measureRows(geometry));
    viewport().scrollTop = 500;
    fireEvent.scroll(viewport());
    viewport().scrollTop = 600;
    fireEvent.scroll(viewport());
    act(() => apiRef.current?.settle());
    geometry.heights.set(5, 260);
    act(() => measureRows(geometry));
    expect(viewport().scrollTop).toBe(600);
    expect(apiRef.current?.canFollow()).toBe(false);
  });

  it.each(["Esc", "outside click"])(
    "replays newest-row menu growth on close by %s",
    async (close) => {
      const apiRef = createRef<AnchorApi>();
      const geometry: Geometry = { ids: [1, 2, 3], heights: new Map() };
      const view = render(<Harness apiRef={apiRef} geometry={geometry} />);

      follow(apiRef, geometry);
      act(() => rowOf(3).focus());
      view.rerender(<Harness apiRef={apiRef} geometry={geometry} popupId={3} />);
      geometry.heights.set(3, 260);
      act(() => measureRows(geometry));
      expect(viewport().scrollTop).toBe(300);

      if (close === "Esc") act(() => rowOf(3).querySelector<HTMLAnchorElement>("a")?.focus());
      else act(() => rowOf(3).blur());

      view.rerender(<Harness apiRef={apiRef} geometry={geometry} />);
      await act(async () => undefined);
      expect(viewport().scrollTop).toBe(360);
      expect(apiRef.current?.canFollow()).toBe(true);
    },
  );

  it("releases row retention after blur", () => {
    const apiRef = createRef<AnchorApi>();
    const geometry: Geometry = { ids: [1, 2, 3], heights: new Map() };

    render(<Harness apiRef={apiRef} geometry={geometry} />);
    follow(apiRef, geometry);
    act(() => rowOf(2).focus());
    expect(apiRef.current?.keepMounted).toEqual([1]);
    act(() => rowOf(2).blur());
    expect(apiRef.current?.keepMounted).toEqual([]);
  });

  it.each(["menu", "editor"])(
    "releases a blurred row when its %s interaction completes",
    async (interaction) => {
      const apiRef = createRef<AnchorApi>();
      const geometry: Geometry = { ids: [1, 2, 3], heights: new Map() };
      const view = render(<Harness apiRef={apiRef} geometry={geometry} />);

      follow(apiRef, geometry);
      act(() => rowOf(2).focus());
      view.rerender(
        <Harness
          apiRef={apiRef}
          geometry={geometry}
          {...(interaction === "menu" ? { popupId: 2 } : { editingId: 2 })}
        />,
      );
      act(() => rowOf(2).blur());
      expect(apiRef.current?.keepMounted).toEqual([1]);
      view.rerender(<Harness apiRef={apiRef} geometry={geometry} />);
      await act(async () => undefined);
      expect(apiRef.current?.keepMounted).toEqual([]);
    },
  );

  it.each(["timers first", "frames first"])(
    "keeps a pressed context row until its deferred popup opens with %s",
    async (order) => {
      const frames: FrameRequestCallback[] = [];
      vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) =>
        frames.push(callback),
      );
      vi.stubGlobal("cancelAnimationFrame", () => undefined);
      vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });

      try {
        const apiRef = createRef<AnchorApi>();
        const geometry: Geometry = { ids: [1, 2, 3], heights: new Map() };
        const view = render(<Harness apiRef={apiRef} geometry={geometry} />);

        follow(apiRef, geometry);
        fireEvent.contextMenu(rowOf(2), { button: 2, buttons: 2 });
        view.rerender(<Harness apiRef={apiRef} geometry={geometry} popupPendingId={2} />);
        // The production popup opens in the task after pointer release.
        window.setTimeout(() => {
          view.rerender(<Harness apiRef={apiRef} geometry={geometry} popupId={2} />);
        }, 0);
        fireEvent(rowOf(2), new Event("pointerup", { bubbles: true }));
        await act(async () => undefined);
        expect(apiRef.current?.keepMounted).toEqual([1]);

        if (order === "frames first") {
          act(() => {
            for (const callback of frames.splice(0)) callback(0);
          });
          expect(apiRef.current?.keepMounted).toEqual([1]);
        }

        await act(async () => vi.runOnlyPendingTimers());
        expect(rowOf(2).querySelector(".message-popup-anchor")).not.toBeNull();
        act(() => {
          for (const callback of frames.splice(0)) callback(0);
        });
        expect(apiRef.current?.keepMounted).toEqual([1]);

        view.rerender(<Harness apiRef={apiRef} geometry={geometry} />);
        await act(async () => undefined);
        expect(apiRef.current?.keepMounted).toEqual([]);
      } finally {
        vi.useRealTimers();
      }
    },
  );

  it.each(["task", "frame"])("cancels the retention release %s on unmount", (phase) => {
    const frames = new Map<number, FrameRequestCallback>();
    let nextFrame = 0;
    vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => {
      const id = ++nextFrame;

      frames.set(id, callback);

      return id;
    });
    vi.stubGlobal("cancelAnimationFrame", (id: number) => frames.delete(id));
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });

    try {
      const apiRef = createRef<AnchorApi>();
      const geometry: Geometry = { ids: [1, 2, 3], heights: new Map() };
      const view = render(<Harness apiRef={apiRef} geometry={geometry} />);

      follow(apiRef, geometry);
      fireEvent.contextMenu(rowOf(2), { button: 2, buttons: 2 });
      fireEvent(rowOf(2), new Event("pointerup", { bubbles: true }));

      if (phase === "frame") act(() => vi.runOnlyPendingTimers());
      view.unmount();
      expect(vi.getTimerCount()).toBe(0);
      expect(frames.size).toBe(0);
    } finally {
      vi.useRealTimers();
    }
  });

  it("retains pending popup ownership after both the release task and frame", async () => {
    const frames: FrameRequestCallback[] = [];
    vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) =>
      frames.push(callback),
    );
    vi.stubGlobal("cancelAnimationFrame", () => undefined);
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });

    try {
      const apiRef = createRef<AnchorApi>();
      const geometry: Geometry = { ids: [1, 2, 3], heights: new Map() };
      const view = render(<Harness apiRef={apiRef} geometry={geometry} />);

      follow(apiRef, geometry);
      fireEvent.contextMenu(rowOf(2), { button: 2, buttons: 2 });
      view.rerender(<Harness apiRef={apiRef} geometry={geometry} popupPendingId={2} />);
      fireEvent(rowOf(2), new Event("pointerup", { bubbles: true }));
      act(() => vi.runOnlyPendingTimers());
      act(() => {
        for (const callback of frames.splice(0)) callback(0);
      });
      expect(apiRef.current?.keepMounted).toEqual([1]);

      // The popup commits after all release work, transferring ownership without a gap.
      view.rerender(<Harness apiRef={apiRef} geometry={geometry} popupId={2} />);
      await act(async () => undefined);
      expect(rowOf(2).querySelector(".message-popup-anchor")).not.toBeNull();
      expect(apiRef.current?.keepMounted).toEqual([1]);
      view.rerender(<Harness apiRef={apiRef} geometry={geometry} />);
      await act(async () => undefined);
      expect(apiRef.current?.keepMounted).toEqual([]);
    } finally {
      vi.useRealTimers();
    }
  });

  it("viewport shrink without row displacement preserves follow", () => {
    const apiRef = createRef<AnchorApi>();
    const geometry: Geometry = { ids: [1, 2, 3], heights: new Map() };

    render(<Harness apiRef={apiRef} geometry={geometry} />);
    act(() => MeasuringObserver.deliver([[viewport(), VIEWPORT_HEIGHT]]));
    follow(apiRef, geometry);
    const element = viewport();

    Object.defineProperty(element, "clientHeight", { configurable: true, value: 200 });
    element.getBoundingClientRect = () => new DOMRect(0, 0, 600, 200);
    act(() => MeasuringObserver.deliver([[element, 200]]));
    expect(element.scrollTop).toBe(400);
    expect(apiRef.current?.canFollow()).toBe(true);
  });

  it("replays deferred end correction when the interacting newest row unmounts", async () => {
    const apiRef = createRef<AnchorApi>();
    const geometry: Geometry = { ids: [1, 2, 3], heights: new Map() };
    const view = render(<Harness apiRef={apiRef} geometry={geometry} />);

    follow(apiRef, geometry);
    act(() => rowOf(3).focus());
    view.rerender(<Harness apiRef={apiRef} geometry={geometry} popupId={3} />);
    geometry.heights.set(2, 500);
    act(() => measureRows(geometry));
    geometry.ids = [1, 2];
    view.rerender(<Harness apiRef={apiRef} geometry={geometry} />);
    await act(async () => undefined);
    expect(viewport().scrollTop).toBe(400);
    expect(apiRef.current?.keepMounted).toEqual([]);
    expect(apiRef.current?.canFollow()).toBe(true);
  });

  it("ignores a cached row's first measurement after settle but follows subsequent growth", async () => {
    const apiRef = createRef<AnchorApi>();
    const geometry: Geometry = { ids: [1, 2, 3], heights: new Map() };
    const view = render(<Harness apiRef={apiRef} geometry={geometry} drawn={[2, 3]} />);

    follow(apiRef, geometry);
    expect(apiRef.current?.canFollow()).toBe(true);

    // Virtua mounts a previously measured row without the reader moving.
    view.rerender(<Harness apiRef={apiRef} geometry={geometry} />);
    await act(async () => undefined);
    act(() => measureRows(geometry));
    expect(viewport().scrollTop).toBe(300);

    // A real image/font resize of a row already being observed still keeps the bottom pinned.
    geometry.heights.set(3, 260);
    act(() => measureRows(geometry));
    expect(viewport().scrollTop).toBe(360);
  });

  it("relinquishes follow when an older row takes focus and retains its identity through prepend", () => {
    const apiRef = createRef<AnchorApi>();
    const geometry: Geometry = { ids: [1, 2, 3], heights: new Map() };
    const view = render(<Harness apiRef={apiRef} geometry={geometry} />);

    follow(apiRef, geometry);
    act(() => rowOf(2).focus());
    expect(apiRef.current?.canFollow()).toBe(false);
    expect(apiRef.current?.keepMounted).toEqual([1]);

    geometry.ids = [0, 1, 2, 3];
    view.rerender(<Harness apiRef={apiRef} geometry={geometry} />);
    expect(apiRef.current?.keepMounted).toEqual([2]);
    expect(document.activeElement).toBe(rowOf(2));

    geometry.heights.set(3, 260);
    act(() => measureRows(geometry));
    expect(viewport().scrollTop).toBe(300);
  });

  it.each(["End", "PageDown"])("%s navigation at the bottom preserves follow intent", (key) => {
    const apiRef = createRef<AnchorApi>();
    const geometry: Geometry = { ids: [1, 2, 3], heights: new Map() };

    render(<Harness apiRef={apiRef} geometry={geometry} />);
    follow(apiRef, geometry);
    fireEvent.keyDown(viewport(), { key });
    expect(apiRef.current?.canFollow()).toBe(true);

    geometry.heights.set(3, 260);
    act(() => measureRows(geometry));
    expect(viewport().scrollTop).toBe(360);
  });

  it("End in a fitting list follows growth during reveal", () => {
    const apiRef = createRef<AnchorApi>();

    const geometry: Geometry = {
      ids: [1, 2],
      heights: new Map([
        [1, 200],
        [2, 100],
      ]),
    };

    render(<Harness apiRef={apiRef} geometry={geometry} cardsLoaded={false} hasCards />);
    follow(apiRef, geometry);
    fireEvent.keyDown(viewport(), { key: "End" });
    expect(apiRef.current?.canFollow()).toBe(true);

    geometry.heights.set(1, 260);
    act(() => measureRows(geometry));
    expect(viewport().scrollTop).toBe(60);
    expect(rowOf(2).getBoundingClientRect().top).toBe(200);
    expect(apiRef.current?.canFollow()).toBe(true);
  });

  it.each(["focus", "Shift+Tab", "click"])(
    "an older control's %s keeps its row through another member's append",
    (input) => {
      const apiRef = createRef<AnchorApi>();
      const geometry: Geometry = { ids: [1, 2, 3], heights: new Map() };
      const view = render(<Harness apiRef={apiRef} geometry={geometry} />);

      follow(apiRef, geometry);
      const control = rowOf(1).querySelector("a");

      if (!control) throw new Error("The older control was not drawn");

      if (input === "Shift+Tab") fireEvent.keyDown(rowOf(3), { key: "Tab", shiftKey: true });

      if (input === "click") fireEvent.pointerDown(control, { button: 0 });
      act(() => control.focus());
      viewport().scrollTop = 0;
      fireEvent.scroll(viewport());
      expect(apiRef.current?.canFollow()).toBe(false);

      geometry.ids = [1, 2, 3, 4];
      view.rerender(<Harness apiRef={apiRef} geometry={geometry} />);
      act(() => measureRows(geometry));
      expect(apiRef.current?.canFollow()).toBe(false);
      expect(viewport().scrollTop).toBe(0);
      expect(document.activeElement).toBe(control);
    },
  );

  it.each(["scroll event", "resize before scroll event", "follow check before scroll event"])(
    "find-in-page relinquishes follow with a %s",
    (delivery) => {
      const apiRef = createRef<AnchorApi>();
      const geometry: Geometry = { ids: [1, 2, 3], heights: new Map() };

      render(<Harness apiRef={apiRef} geometry={geometry} />);
      follow(apiRef, geometry);
      viewport().scrollTop = 100;

      if (delivery === "scroll event") fireEvent.scroll(viewport());

      if (delivery === "follow check before scroll event")
        expect(apiRef.current?.canFollow()).toBe(false);

      geometry.heights.set(2, 260);
      act(() => measureRows(geometry));
      expect(viewport().scrollTop).toBe(100);
      expect(apiRef.current?.canFollow()).toBe(false);
    },
  );

  it.each(["mousedown", "auxclick"])(
    "middle-button %s cancels end intent before autoscroll",
    (event) => {
      const apiRef = createRef<AnchorApi>();
      const geometry: Geometry = { ids: [1, 2, 3], heights: new Map() };

      render(<Harness apiRef={apiRef} geometry={geometry} />);
      follow(apiRef, geometry);
      fireEvent(viewport(), new MouseEvent(event, { button: 1, bubbles: true }));
      expect(apiRef.current?.canFollow()).toBe(false);
      viewport().scrollTop = 100;
      fireEvent.scroll(viewport());
      geometry.heights.set(2, 260);
      act(() => measureRows(geometry));
      expect(viewport().scrollTop).toBe(100);
    },
  );

  it("a late middle-button release preserves follow after autoscroll reaches the bottom", () => {
    const apiRef = createRef<AnchorApi>();
    const geometry: Geometry = { ids: [1, 2, 3], heights: new Map() };

    render(<Harness apiRef={apiRef} geometry={geometry} />);
    follow(apiRef, geometry);
    fireEvent.mouseDown(viewport(), { button: 1 });
    viewport().scrollTop = 100;
    fireEvent.scroll(viewport());
    viewport().scrollTop = 300;
    fireEvent.scroll(viewport());
    fireEvent(viewport(), new MouseEvent("auxclick", { button: 1, bubbles: true }));
    expect(apiRef.current?.canFollow()).toBe(true);
  });

  it("find-in-page can interrupt an issued smooth follow before it reaches the end", () => {
    const apiRef = createRef<AnchorApi>();
    const geometry: Geometry = { ids: [1, 2, 3, 4, 5], heights: new Map() };

    render(<Harness apiRef={apiRef} geometry={geometry} />);
    viewport().scrollTop = 100;
    act(() => apiRef.current?.followEnd());
    viewport().scrollTop = 500;
    fireEvent.scroll(viewport());
    expect(apiRef.current?.canFollow()).toBe(true);
    viewport().scrollTop = 300;
    fireEvent.scroll(viewport());
    act(() => apiRef.current?.settle());
    expect(apiRef.current?.canFollow()).toBe(false);
    geometry.heights.set(3, 260);
    act(() => measureRows(geometry));
    expect(viewport().scrollTop).toBe(300);
  });

  it("a layout shrink that clamps the bottom preserves follow before scroll delivery", () => {
    const apiRef = createRef<AnchorApi>();
    const geometry: Geometry = { ids: [1, 2, 3], heights: new Map() };

    render(<Harness apiRef={apiRef} geometry={geometry} />);
    follow(apiRef, geometry);
    geometry.heights.set(3, 150);
    viewport().scrollTop = 250;
    expect(apiRef.current?.canFollow()).toBe(true);
    fireEvent.scroll(viewport());
    expect(apiRef.current?.canFollow()).toBe(true);
  });

  it("a delivered layout shrink preserves follow at the clamped end", () => {
    const apiRef = createRef<AnchorApi>();
    const geometry: Geometry = { ids: [1, 2, 3], heights: new Map() };

    render(<Harness apiRef={apiRef} geometry={geometry} />);
    follow(apiRef, geometry);
    fireEvent.scroll(viewport());
    geometry.heights.set(3, 150);
    viewport().scrollTop = 250;
    act(() => measureRows(geometry));
    expect(apiRef.current?.canFollow()).toBe(true);
    geometry.heights.set(3, 260);
    act(() => measureRows(geometry));
    expect(viewport().scrollTop).toBe(360);
  });

  it("growth during an issued jump waits for settlement and then follows", () => {
    const apiRef = createRef<AnchorApi>();
    const geometry: Geometry = { ids: [1, 2, 3, 4, 5], heights: new Map() };

    render(<Harness apiRef={apiRef} geometry={geometry} />);
    viewport().scrollTop = 100;
    act(() => apiRef.current?.followEnd());
    act(() => measureRows(geometry));
    viewport().scrollTop = 500;
    fireEvent.scroll(viewport());
    geometry.heights.set(5, 260);
    act(() => measureRows(geometry));
    expect(viewport().scrollTop).toBe(500);
    expect(apiRef.current?.canFollow()).toBe(true);
    // Native animation completes at the destination issued before that growth.
    viewport().scrollTop = 700;
    fireEvent.scroll(viewport());
    act(() => apiRef.current?.settle());
    expect(viewport().scrollTop).toBe(760);
    expect(apiRef.current?.canFollow()).toBe(true);
  });

  it("Virtua's 150ms debounce settles deferred growth without native scrollend", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout", "Date"] });

    try {
      // Virtua captures setTimeout at import, so load it after installing the fake clock.
      const { VList } = await import("virtua");
      const apiRef = createRef<AnchorApi>();
      const geometry: Geometry = { ids: [1, 2, 3, 4, 5], heights: new Map() };
      const settled = vi.fn();

      render(<Harness apiRef={apiRef} geometry={geometry} virtual={VList} onSettled={settled} />);
      act(() => MeasuringObserver.deliver([[viewport(), VIEWPORT_HEIGHT]]));
      await act(async () => undefined);
      act(() => measureRows(geometry));
      viewport().scrollTop = 100;
      act(() => apiRef.current?.followEnd());
      viewport().scrollTop = 500;
      fireEvent.scroll(viewport());
      await act(async () => undefined);
      act(() => measureRows(geometry));
      geometry.heights.set(5, 260);
      act(() => measureRows(geometry));
      expect(viewport().scrollTop).toBe(500);
      viewport().scrollTop = 700;
      fireEvent.scroll(viewport());
      act(() => vi.advanceTimersByTime(149));
      expect(settled).not.toHaveBeenCalled();
      expect(viewport().scrollTop).toBe(700);
      act(() => vi.advanceTimersByTime(1));
      expect(settled).toHaveBeenCalledOnce();
      expect(viewport().scrollTop).toBe(760);
      expect(apiRef.current?.canFollow()).toBe(true);
    } finally {
      vi.useRealTimers();
    }
  });

  it("an issued jump settles at the actual end after its destination shrinks", () => {
    const apiRef = createRef<AnchorApi>();
    const geometry: Geometry = { ids: [1, 2, 3, 4, 5], heights: new Map() };

    render(<Harness apiRef={apiRef} geometry={geometry} />);
    viewport().scrollTop = 100;
    act(() => apiRef.current?.followEnd());
    act(() => measureRows(geometry));
    geometry.heights.set(5, 150);
    viewport().scrollTop = 650;
    act(() => measureRows(geometry));
    // Native scrollend can precede Virtua's scroll/capture callback.
    fireEvent(viewport(), new Event("scrollend"));
    expect(apiRef.current?.canFollow()).toBe(true);
    geometry.heights.set(5, 260);
    act(() => measureRows(geometry));
    expect(viewport().scrollTop).toBe(760);
  });

  it("returning to the end restores follow before its scroll callback", () => {
    const apiRef = createRef<AnchorApi>();
    const geometry: Geometry = { ids: [1, 2, 3], heights: new Map() };

    render(<Harness apiRef={apiRef} geometry={geometry} />);
    follow(apiRef, geometry);
    viewport().scrollTop = 100;
    fireEvent.scroll(viewport());
    viewport().scrollTop = 300;
    expect(apiRef.current?.canFollow()).toBe(true);
  });

  it("each correction rebases growth before the next eventless reader movement", () => {
    const apiRef = createRef<AnchorApi>();
    const geometry: Geometry = { ids: [1, 2, 3, 4, 5], heights: new Map() };

    render(<Harness apiRef={apiRef} geometry={geometry} />);
    viewport().scrollTop = 100;
    act(() => apiRef.current?.followEnd());
    act(() => measureRows(geometry));
    // The app's issued jump reaches the end; later growth has its own budget.
    viewport().scrollTop = 700;
    fireEvent.scroll(viewport());
    geometry.heights.set(4, 260);
    act(() => measureRows(geometry));
    expect(viewport().scrollTop).toBe(760);
    fireEvent.scroll(viewport());
    geometry.heights.set(5, 500);
    // Above the last pin even after deducting this growth. The previous 60px
    // correction must not remain available to explain this reader movement.
    viewport().scrollTop = 700;
    expect(apiRef.current?.canFollow()).toBe(false);
  });

  it("retains a reader row while Virtua partially compensates card growth", () => {
    const apiRef = createRef<AnchorApi>();
    const geometry: Geometry = { ids: [1, 2, 3], heights: new Map() };

    const view = render(
      <Harness apiRef={apiRef} geometry={geometry} cardsLoaded={false} hasCards />,
    );

    follow(apiRef, geometry);
    fireEvent.wheel(viewport(), { deltaY: -200 });
    viewport().scrollTop = 100;
    fireEvent.scroll(viewport());
    expect(rowOf(2).getBoundingClientRect().top).toBe(100);
    expect(apiRef.current?.canFollow()).toBe(false);

    geometry.heights.set(1, 300);
    // The virtualiser corrects part of the growth before our observer delivery.
    viewport().scrollTop = 150;
    view.rerender(<Harness apiRef={apiRef} geometry={geometry} cardsLoaded hasCards />);
    act(() => measureRows(geometry));
    expect(viewport().scrollTop).toBe(200);
    expect(rowOf(2).getBoundingClientRect().top).toBe(100);
    fireEvent.scroll(viewport());
    expect(apiRef.current?.canFollow()).toBe(false);
  });

  it.each(["End", "wheel", "external scroll", "jump"])(
    "returning to the bottom by %s restores follow",
    (input) => {
      const apiRef = createRef<AnchorApi>();
      const geometry: Geometry = { ids: [1, 2, 3], heights: new Map() };

      render(<Harness apiRef={apiRef} geometry={geometry} />);
      follow(apiRef, geometry);
      viewport().scrollTop = 100;
      fireEvent.scroll(viewport());
      expect(apiRef.current?.canFollow()).toBe(false);

      if (input === "End") fireEvent.keyDown(viewport(), { key: "End" });

      if (input === "wheel") fireEvent.wheel(viewport(), { deltaY: 200 });

      if (input === "jump") act(() => apiRef.current?.followEnd());
      viewport().scrollTop = 300;
      fireEvent.scroll(viewport());
      act(() => rowOf(3).focus());
      expect(apiRef.current?.canFollow()).toBe(true);
      geometry.heights.set(3, 260);
      act(() => measureRows(geometry));
      expect(viewport().scrollTop).toBe(360);
      fireEvent.scroll(viewport());
      expect(apiRef.current?.canFollow()).toBe(true);
    },
  );

  it.each([
    { interaction: "menu", cardsLoaded: true },
    { interaction: "editor", cardsLoaded: true },
    { interaction: "menu", cardsLoaded: false },
    { interaction: "editor", cardsLoaded: false },
  ])(
    "blocks growth correction with a $interaction open (cards loaded: $cardsLoaded)",
    (scenario) => {
      const apiRef = createRef<AnchorApi>();
      const geometry: Geometry = { ids: [1, 2, 3], heights: new Map() };

      render(
        <Harness
          apiRef={apiRef}
          geometry={geometry}
          cardsLoaded={scenario.cardsLoaded}
          hasCards={!scenario.cardsLoaded}
          {...(scenario.interaction === "menu" ? { popupId: 2 } : { editingId: 2 })}
        />,
      );
      follow(apiRef, geometry);
      expect(apiRef.current?.canFollow()).toBe(false);

      geometry.heights.set(3, 260);
      act(() => measureRows(geometry));
      expect(viewport().scrollTop).toBe(300);
    },
  );

  it("retains a right-clicked row before the menu opens and remaps it when a page prepends", () => {
    const apiRef = createRef<AnchorApi>();
    const geometry: Geometry = { ids: [1, 2, 3], heights: new Map() };
    const view = render(<Harness apiRef={apiRef} geometry={geometry} />);

    follow(apiRef, geometry);
    fireEvent.contextMenu(rowOf(2).firstElementChild ?? rowOf(2), { button: 2 });
    expect(apiRef.current?.canFollow()).toBe(false);
    expect(apiRef.current?.keepMounted).toEqual([1]);

    geometry.ids = [0, 1, 2, 3];
    view.rerender(<Harness apiRef={apiRef} geometry={geometry} popupId={2} />);
    expect(apiRef.current?.keepMounted).toEqual([2]);
  });
});
