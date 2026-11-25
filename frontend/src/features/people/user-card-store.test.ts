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
  isUserCardOpening,
  OPEN_DELAY_MS,
  openByClick,
  pinUserCard,
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
  document.body.replaceChildren();
});

/** An element in the document, as a mounted trigger is. */
function mounted(tag = "span"): HTMLElement {
  return document.body.appendChild(document.createElement(tag));
}

/** The opening the card shows, or `null` when none does. */
function openingId(): number | null {
  const state = getUserCardState();

  return state.kind === "open" || state.kind === "closing" ? state.openingId : null;
}

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
    const body = mounted("div");
    const mention = document.createElement("span");

    body.append(mention);
    openByClick(target(8, mention));
    releaseAnchor(mounted("div"));
    expect(shown()).toBe("open:8");

    releaseAnchor(body);
    expect(shown()).toBe("closed");
  });

  it("closes when its anchor already left the document, as a replaced mention has", () => {
    const body = mounted("div");
    const mention = document.createElement("span");

    body.append(mention);
    openByClick(target(8, mention));
    mention.remove();
    releaseAnchor(body);

    expect(shown()).toBe("closed");
  });

  it("calls off a swap to a trigger going away, and the card it would replace still closes", () => {
    const first = mounted();
    const second = mounted();

    hoverEnter(target(8, first), "mouse");
    vi.advanceTimersByTime(OPEN_DELAY_MS);
    hoverEnter(target(9, second), "mouse");
    releaseAnchor(second);
    vi.advanceTimersByTime(WARM_DELAY_MS * 3);
    expect(shown()).toBe("open:8");

    hoverLeave();
    hoverEnter(target(9, second), "mouse");
    expect(shown()).toBe("closing:8");
    releaseAnchor(second);
    vi.advanceTimersByTime(CLOSE_DELAY_MS - 1);
    expect(shown()).toBe("closing:8");
    vi.advanceTimersByTime(1);
    expect(shown()).toBe("closed");
  });

  it("numbers every opening, even of the same person's card, and keeps it while it shows", () => {
    const anchor = mounted();

    openByClick(target(8, anchor));

    const first = openingId();

    hoverLeave();
    openByClick(target(8, mounted()));
    expect(openingId()).toBe(first);
    expect(isUserCardOpening(first ?? 0)).toBe(true);

    closeUserCard();
    expect(isUserCardOpening(first ?? 0)).toBe(false);

    openByClick(target(8, anchor));
    expect(openingId()).not.toBe(first);
    expect(isUserCardOpening(first ?? 0)).toBe(false);

    const second = openingId();

    hoverEnter(target(9), "mouse");
    closeUserCard();
    hoverEnter(target(9), "mouse");
    vi.advanceTimersByTime(OPEN_DELAY_MS);
    expect(openingId()).not.toBe(second);
  });

  it("pins a hover-opened card through the pointer leaving it", () => {
    hoverEnter(target(8), "mouse");
    vi.advanceTimersByTime(OPEN_DELAY_MS);

    const opening = openingId();

    cardEnter();
    pinUserCard();
    cardLeave();
    vi.advanceTimersByTime(CLOSE_DELAY_MS * 2);

    expect(getUserCardState()).toMatchObject({ kind: "open", via: "click", openingId: opening });
  });

  it("pins a card in its close grace, and pins nothing when none shows", () => {
    hoverEnter(target(8), "mouse");
    vi.advanceTimersByTime(OPEN_DELAY_MS);
    hoverLeave();
    pinUserCard();
    vi.advanceTimersByTime(CLOSE_DELAY_MS * 2);
    expect(shown()).toBe("open:8");

    closeUserCard();
    pinUserCard();
    expect(shown()).toBe("closed");
  });
});
