import { act, fireEvent, render, screen } from "@testing-library/react";
import { useRef } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { LONG_PRESS_MS } from "../../lib/long-press.ts";
import { useRowGestures } from "./row-gestures.ts";

interface RowProps {
  readonly onSwipe: () => void;
  readonly onLongPress: (x: number, y: number) => void;
  readonly onOpen: () => void;
}

function Row({ onSwipe, onLongPress, onOpen }: RowProps) {
  const rowRef = useRef<HTMLDivElement | null>(null);
  const innerRef = useRef<HTMLDivElement | null>(null);

  const gestures = useRowGestures(
    rowRef,
    innerRef,
    { label: "Done", icon: "circle-check", tone: "success", onSwipe },
    onLongPress,
  );

  return (
    <div ref={rowRef} data-testid="row" {...gestures.props}>
      <div ref={innerRef}>
        <button type="button" onClick={onOpen}>
          Open
        </button>
      </div>
    </div>
  );
}

const POINTER = { pointerId: 4, pointerType: "touch", isPrimary: true, clientY: 20 } as const;

/** A finger at `x` on the row's button (the events bubble to the row's handlers). */
function finger(type: "pointerDown" | "pointerMove" | "pointerUp", x: number) {
  fireEvent[type](screen.getByRole("button", { name: "Open" }), { ...POINTER, clientX: x });
}

/** Drags the finger from 300 px to 100 px in 10 px steps. */
function dragLeft() {
  for (let x = 290; x >= 100; x -= 10) {
    finger("pointerMove", x);
  }
}

function setup() {
  const handlers = { onSwipe: vi.fn(), onLongPress: vi.fn(), onOpen: vi.fn() };

  render(<Row {...handlers} />);

  // jsdom lays nothing out: give the row a width, so 200 px is past the swipe's line.
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue(
    DOMRect.fromRect({ x: 0, y: 0, width: 360, height: 60 }),
  );

  return handlers;
}

describe("useRowGestures", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("a press held past the long press opens the menu on release, without the click", () => {
    const { onSwipe, onLongPress, onOpen } = setup();

    finger("pointerDown", 300);
    act(() => vi.advanceTimersByTime(LONG_PRESS_MS + 10));
    expect(onLongPress).not.toHaveBeenCalled();
    finger("pointerUp", 300);
    fireEvent.click(screen.getByRole("button", { name: "Open" }), { detail: 1 });
    act(() => vi.runOnlyPendingTimers());

    expect(onLongPress).toHaveBeenCalledExactlyOnceWith(300, 20);
    expect(onSwipe).not.toHaveBeenCalled();
    expect(onOpen).not.toHaveBeenCalled();
  });

  it("a held press that then slides sideways stays a long press: no swipe", () => {
    const { onSwipe, onLongPress } = setup();

    finger("pointerDown", 300);
    act(() => vi.advanceTimersByTime(LONG_PRESS_MS + 10));
    dragLeft();
    expect(screen.getByTestId("row").dataset.swipe).toBeUndefined();
    finger("pointerUp", 100);
    act(() => vi.runAllTimers());

    expect(onSwipe).not.toHaveBeenCalled();
    expect(onLongPress).toHaveBeenCalledOnce();
  });

  it("a swipe that then rests past the long press's time stays a swipe: no menu", () => {
    const { onSwipe, onLongPress, onOpen } = setup();

    finger("pointerDown", 300);
    finger("pointerMove", 280);
    expect(screen.getByTestId("row").dataset.swipe).toBe("dragging");
    act(() => vi.advanceTimersByTime(LONG_PRESS_MS * 3));
    dragLeft();
    finger("pointerUp", 100);
    fireEvent.click(screen.getByRole("button", { name: "Open" }), { detail: 1 });
    act(() => vi.runAllTimers());

    expect(onSwipe).toHaveBeenCalledOnce();
    expect(onLongPress).not.toHaveBeenCalled();
    expect(onOpen).not.toHaveBeenCalled();
  });

  it("a quick tap opens the row", () => {
    const { onSwipe, onLongPress, onOpen } = setup();

    finger("pointerDown", 300);
    finger("pointerUp", 300);
    fireEvent.click(screen.getByRole("button", { name: "Open" }), { detail: 1 });
    act(() => vi.runAllTimers());

    expect(onOpen).toHaveBeenCalledOnce();
    expect(onSwipe).not.toHaveBeenCalled();
    expect(onLongPress).not.toHaveBeenCalled();
  });
});
