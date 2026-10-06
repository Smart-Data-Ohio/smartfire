import { render } from "@testing-library/react";
import { useRef } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { messageFixture } from "../../api/testing.ts";
import type { TimelineItem } from "../room/timeline-items.ts";
import { requestListEdge, useListEdges } from "./list-edges.ts";

/** One frame of fake time. */
const FRAME_MS = 16;

function items(ids: readonly number[]): TimelineItem[] {
  return ids.map((id) => ({
    kind: "message",
    key: `c-${id}`,
    message: messageFixture(id, 12),
    groupStart: true,
  }));
}

interface HarnessProps {
  /** The list's messages, in order. */
  readonly ids: readonly number[];
  /** The rows the virtualiser has drawn. */
  readonly drawn: readonly number[];
  readonly scrollToIndex: (index: number) => void;
}

/** A list as the timelines lay it out, with a stand-in virtualiser that only records scrolls. */
function Harness({ ids, drawn, scrollToIndex }: HarnessProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const listRef = useRef({ scrollToIndex });

  useListEdges(containerRef, listRef, items(ids));

  return (
    <div ref={containerRef}>
      <div role="log" aria-label="Messages" tabIndex={-1}>
        {drawn.map((id) => (
          <div key={id} data-message-row data-message-id={id} tabIndex={-1} />
        ))}
      </div>
    </div>
  );
}

function log(): HTMLElement | null {
  return document.querySelector<HTMLElement>('[role="log"]');
}

function rowOf(id: number): HTMLElement | null {
  return document.querySelector<HTMLElement>(`[data-message-id="${id}"]`);
}

/** Lets `count` frames run. */
function frames(count: number): void {
  vi.advanceTimersByTime(count * FRAME_MS);
}

describe("useListEdges", () => {
  beforeEach(() => {
    vi.useFakeTimers({ toFake: ["requestAnimationFrame", "cancelAnimationFrame", "performance"] });
    // jsdom doesn't lay out, so it has no scrollIntoView; a focused edge row scrolls itself in.
    Element.prototype.scrollIntoView = () => undefined;
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("holds focus on the list until the edge row is drawn, however many frames that takes", () => {
    const scrolls: number[] = [];
    const scrollToIndex = (index: number) => scrolls.push(index);
    const view = render(<Harness ids={[1, 2, 3]} drawn={[3]} scrollToIndex={scrollToIndex} />);

    rowOf(3)?.focus();
    expect(requestListEdge(rowOf(3) ?? document.body, "first")).toBe(true);
    expect(scrolls).toEqual([0]);
    // Off the row the key moved away from at once.
    expect(document.activeElement).toBe(log());

    // The scroll unmounts the row that had focus; a page lands before the first row is drawn.
    view.rerender(<Harness ids={[1, 2, 3]} drawn={[]} scrollToIndex={scrollToIndex} />);
    frames(1);
    expect(document.activeElement).toBe(log());

    frames(60);
    view.rerender(<Harness ids={[1, 2, 3]} drawn={[1]} scrollToIndex={scrollToIndex} />);
    frames(1);
    expect(document.activeElement).toBe(rowOf(1));
  });

  it("scrolls again to where a page that landed meanwhile moved the row", () => {
    const scrolls: number[] = [];
    const scrollToIndex = (index: number) => scrolls.push(index);
    const view = render(<Harness ids={[4, 5, 6]} drawn={[4]} scrollToIndex={scrollToIndex} />);

    rowOf(4)?.focus();
    requestListEdge(rowOf(4) ?? document.body, "last");

    // A newer page lands below the window before 6's row is drawn: 6 is still at 2.
    view.rerender(<Harness ids={[4, 5, 6, 7]} drawn={[4]} scrollToIndex={scrollToIndex} />);
    frames(1);
    expect(scrolls).toEqual([2]);

    // An older page lands above it: 6 moves to 5.
    view.rerender(
      <Harness ids={[1, 2, 3, 4, 5, 6, 7]} drawn={[4]} scrollToIndex={scrollToIndex} />,
    );
    frames(1);
    expect(scrolls).toEqual([2, 5]);

    view.rerender(
      <Harness ids={[1, 2, 3, 4, 5, 6, 7]} drawn={[6]} scrollToIndex={scrollToIndex} />,
    );
    frames(1);
    expect(document.activeElement).toBe(rowOf(6));
  });

  it("focuses an edge row that's already drawn at once", () => {
    render(<Harness ids={[1, 2, 3]} drawn={[1, 3]} scrollToIndex={() => undefined} />);

    rowOf(1)?.focus();
    requestListEdge(rowOf(1) ?? document.body, "last");

    expect(document.activeElement).toBe(rowOf(3));
  });

  it("leaves focus on the list, not the page, when the row never comes", () => {
    const scrolls: number[] = [];
    const scrollToIndex = (index: number) => scrolls.push(index);
    const view = render(<Harness ids={[1, 2, 3]} drawn={[3]} scrollToIndex={scrollToIndex} />);

    rowOf(3)?.focus();
    requestListEdge(rowOf(3) ?? document.body, "first");
    view.rerender(<Harness ids={[1, 2, 3]} drawn={[]} scrollToIndex={scrollToIndex} />);
    // 400 frames is about 6.4 s, past the 3 s the wait gives up after.
    frames(400);

    expect(document.activeElement).toBe(log());
    expect(vi.getTimerCount()).toBe(0);
  });

  it("stops waiting once the reader moves focus elsewhere", () => {
    const scrolls: number[] = [];
    const scrollToIndex = (index: number) => scrolls.push(index);
    const view = render(<Harness ids={[1, 2, 3]} drawn={[3]} scrollToIndex={scrollToIndex} />);
    const search = document.createElement("input");

    document.body.append(search);
    rowOf(3)?.focus();
    requestListEdge(rowOf(3) ?? document.body, "first");
    view.rerender(<Harness ids={[1, 2, 3]} drawn={[]} scrollToIndex={scrollToIndex} />);
    frames(1);
    search.focus();
    frames(1);

    expect(document.activeElement).toBe(search);
    expect(vi.getTimerCount()).toBe(0);

    // The row being drawn later doesn't take focus back either.
    view.rerender(<Harness ids={[1, 2, 3]} drawn={[1]} scrollToIndex={scrollToIndex} />);
    frames(10);
    expect(document.activeElement).toBe(search);
    search.remove();
  });

  it("stops waiting when the list unmounts", () => {
    const view = render(<Harness ids={[1, 2, 3]} drawn={[3]} scrollToIndex={() => undefined} />);

    rowOf(3)?.focus();
    requestListEdge(rowOf(3) ?? document.body, "first");
    view.rerender(<Harness ids={[1, 2, 3]} drawn={[]} scrollToIndex={() => undefined} />);
    frames(1);
    view.unmount();

    expect(vi.getTimerCount()).toBe(0);
  });
});
