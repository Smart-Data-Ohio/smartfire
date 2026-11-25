import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  type CardTarget,
  CLOSE_DELAY_MS,
  cardEnter,
  cardLeave,
  closeUserCard,
  getUserCardState,
  hoverEnter,
  hoverLeave,
  OPEN_DELAY_MS,
  openByClick,
  releaseAnchor,
  resetUserCard,
  WARM_DELAY_MS,
} from "./user-card-store.ts";

function target(userId: number, anchor = document.createElement("span")): CardTarget {
  return { userId, anchor, placement: "bottom-start" };
}

/** The card's kind and whose it is, for compact assertions. */
function shown(): string {
  const state = getUserCardState();

  return state.kind === "closed" ? "closed" : `${state.kind}:${state.userId}`;
}

beforeEach(() => {
  vi.useFakeTimers();
  resetUserCard();
});

afterEach(() => {
  resetUserCard();
  vi.useRealTimers();
});

describe("user card lifecycle", () => {
  it("waits out the cold delay before a hover opens a card", () => {
    hoverEnter(target(8), "mouse");
    expect(shown()).toBe("pending:8");

    vi.advanceTimersByTime(OPEN_DELAY_MS - 1);
    expect(shown()).toBe("pending:8");

    vi.advanceTimersByTime(1);
    expect(shown()).toBe("open:8");
    expect(getUserCardState()).toMatchObject({ via: "hover" });
  });

  it("never opens for a pointer that passes over before the delay", () => {
    hoverEnter(target(8), "mouse");
    vi.advanceTimersByTime(OPEN_DELAY_MS / 2);
    hoverLeave();
    vi.advanceTimersByTime(OPEN_DELAY_MS * 2);

    expect(shown()).toBe("closed");
  });

  it("ignores touch and pen hovers", () => {
    hoverEnter(target(8), "touch");
    hoverEnter(target(8), "pen");
    vi.advanceTimersByTime(OPEN_DELAY_MS * 2);

    expect(shown()).toBe("closed");
  });

  it("stays open while the pointer crosses from the trigger onto the card", () => {
    hoverEnter(target(8), "mouse");
    vi.advanceTimersByTime(OPEN_DELAY_MS);
    hoverLeave();
    expect(shown()).toBe("closing:8");

    vi.advanceTimersByTime(CLOSE_DELAY_MS - 1);
    cardEnter();
    vi.advanceTimersByTime(CLOSE_DELAY_MS * 2);
    expect(shown()).toBe("open:8");

    cardLeave();
    vi.advanceTimersByTime(CLOSE_DELAY_MS);
    expect(shown()).toBe("closed");
  });

  it("hands the same person's card from their avatar to their name without closing", () => {
    const avatar = document.createElement("span");
    const name = document.createElement("span");

    hoverEnter(target(8, avatar), "mouse");
    vi.advanceTimersByTime(OPEN_DELAY_MS);
    hoverLeave();
    hoverEnter(target(8, name), "mouse");

    expect(getUserCardState()).toMatchObject({ kind: "open", userId: 8, anchor: name });
    vi.advanceTimersByTime(CLOSE_DELAY_MS * 2);
    expect(shown()).toBe("open:8");
  });

  it("keeps the cold delay running across the same person's triggers, opening off the last", () => {
    const avatar = document.createElement("span");
    const name = document.createElement("span");

    hoverEnter({ userId: 8, anchor: avatar, placement: "right-start" }, "mouse");
    vi.advanceTimersByTime(OPEN_DELAY_MS - 50);
    hoverEnter({ userId: 8, anchor: name, placement: "bottom-start" }, "mouse");
    vi.advanceTimersByTime(50);

    const next = getUserCardState();

    expect(next.kind).toBe("open");
    expect(next.kind === "closed" ? null : next.anchor).toBe(name);
    expect(next.kind === "closed" ? null : next.placement).toBe("bottom-start");
  });

  it("swaps to the last of someone else's triggers the pointer reached during the warm delay", () => {
    const avatar = document.createElement("span");
    const name = document.createElement("span");

    hoverEnter(target(8), "mouse");
    vi.advanceTimersByTime(OPEN_DELAY_MS);
    hoverEnter({ userId: 9, anchor: avatar, placement: "right-start" }, "mouse");
    vi.advanceTimersByTime(WARM_DELAY_MS - 50);
    hoverEnter({ userId: 9, anchor: name, placement: "bottom-start" }, "mouse");
    expect(shown()).toBe("open:8");
    vi.advanceTimersByTime(50);

    const next = getUserCardState();

    expect(shown()).toBe("open:9");
    expect(next.kind === "closed" ? null : next.anchor).toBe(name);
    expect(next.kind === "closed" ? null : next.placement).toBe("bottom-start");
  });

  it("swaps to someone else's card after the warm delay, showing the first until then", () => {
    hoverEnter(target(8), "mouse");
    vi.advanceTimersByTime(OPEN_DELAY_MS);
    hoverLeave();
    hoverEnter(target(9), "mouse");

    vi.advanceTimersByTime(WARM_DELAY_MS - 1);
    expect(getUserCardState()).toMatchObject({ userId: 8 });
    expect(shown()).not.toBe("closed");

    vi.advanceTimersByTime(1);
    expect(shown()).toBe("open:9");
  });

  it("reopens warm just after a card closed", () => {
    hoverEnter(target(8), "mouse");
    vi.advanceTimersByTime(OPEN_DELAY_MS);
    hoverLeave();
    vi.advanceTimersByTime(CLOSE_DELAY_MS);
    expect(shown()).toBe("closed");

    hoverEnter(target(9), "mouse");
    vi.advanceTimersByTime(WARM_DELAY_MS);
    expect(shown()).toBe("open:9");
  });

  it("opens at once on a click, stays through hovers elsewhere, and toggles on its trigger", () => {
    const anchor = document.createElement("button");

    openByClick(target(8, anchor));
    expect(getUserCardState()).toMatchObject({ kind: "open", userId: 8, via: "click" });

    hoverLeave();
    hoverEnter(target(9), "mouse");
    cardLeave();
    vi.advanceTimersByTime(OPEN_DELAY_MS * 2);
    expect(shown()).toBe("open:8");

    openByClick(target(8, anchor));
    expect(shown()).toBe("closed");
  });

  it("pins a hover-opened card when its trigger is clicked", () => {
    const anchor = document.createElement("button");

    hoverEnter(target(8, anchor), "mouse");
    vi.advanceTimersByTime(OPEN_DELAY_MS);
    openByClick(target(8, anchor));
    hoverLeave();
    vi.advanceTimersByTime(CLOSE_DELAY_MS * 2);

    expect(getUserCardState()).toMatchObject({ kind: "open", via: "click" });
  });

  it("closes at once, cancelling a pending open", () => {
    hoverEnter(target(8), "mouse");
    closeUserCard();
    vi.advanceTimersByTime(OPEN_DELAY_MS * 2);
    expect(shown()).toBe("closed");

    openByClick(target(9));
    closeUserCard();
    expect(shown()).toBe("closed");
  });

  it("closes when the element it hangs off goes away", () => {
    const body = document.createElement("div");
    const mention = document.createElement("span");

    body.append(mention);
    openByClick(target(8, mention));
    releaseAnchor(document.createElement("div"));
    expect(shown()).toBe("open:8");

    releaseAnchor(body);
    expect(shown()).toBe("closed");
  });
});
