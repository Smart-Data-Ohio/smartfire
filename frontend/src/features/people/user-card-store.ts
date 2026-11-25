/**
 * The user card's lifecycle (Slack's hover card): one card for the whole app, opened from any
 * avatar or name by hover intent or a click. Hover waits out a cold delay so sweeping the pointer
 * across a timeline never flashes cards; leaving waits out a short grace so the pointer can cross
 * the gap onto the card. Moving between triggers of the same person (an avatar, then the name
 * beside it) just moves the anchor; moving to someone else while a card shows swaps it after a
 * short warm delay, without closing in between. Only a mouse hovers: touch and pen open by tap.
 */
import { useSyncExternalStore } from "react";
import type { Placement } from "../../lib/anchor.ts";

/** How the card came up: hover intent closes on leave, a click stays until dismissed. */
export type CardVia = "hover" | "click";

/** Whose card, and the element it hangs off. */
export interface CardTarget {
  readonly userId: number;
  readonly anchor: HTMLElement;
  readonly placement: Placement;
}

export type UserCardState =
  | { readonly kind: "closed" }
  /** Hover intent: the cold delay is running. Nothing shows yet. */
  | ({ readonly kind: "pending" } & CardTarget)
  | ({ readonly kind: "open"; readonly via: CardVia } & CardTarget)
  /** The pointer left: still showing through the grace delay, back to open if it returns. */
  | ({ readonly kind: "closing"; readonly via: CardVia } & CardTarget);

/** Before a hover opens a card from cold. */
export const OPEN_DELAY_MS = 300;

/** Before a hover swaps the card shown for someone else's, or reopens one just closed. */
export const WARM_DELAY_MS = 100;

/** After the pointer leaves a trigger or the card, before the card closes. */
export const CLOSE_DELAY_MS = 200;

/** How long after a card closes the next hover counts as warm. */
const WARM_WINDOW_MS = 600;

const CLOSED: UserCardState = { kind: "closed" };

let state: UserCardState = CLOSED;

let timer: ReturnType<typeof setTimeout> | undefined;

let closedAt = Number.NEGATIVE_INFINITY;

const listeners = new Set<() => void>();

function set(next: UserCardState): void {
  if (next === state) {
    return;
  }

  if (next.kind === "closed" && state.kind !== "closed" && state.kind !== "pending") {
    closedAt = Date.now();
  }

  state = next;

  for (const listener of listeners) {
    listener();
  }
}

function clearTimer(): void {
  clearTimeout(timer);
  timer = undefined;
}

/** Runs `step` after `ms`, replacing whatever was scheduled. */
function schedule(ms: number, step: () => void): void {
  clearTimer();
  timer = setTimeout(() => {
    timer = undefined;
    step();
  }, ms);
}

/** Whether a card shows (open, or in its close grace). */
function shown(
  current: UserCardState,
): current is Extract<UserCardState, { kind: "open" | "closing" }> {
  return current.kind === "open" || current.kind === "closing";
}

export function getUserCardState(): UserCardState {
  return state;
}

export function subscribeUserCard(listener: () => void): () => void {
  listeners.add(listener);

  return () => listeners.delete(listener);
}

/** The card's state, for the one host and for triggers that mark themselves expanded. */
export function useUserCard(): UserCardState {
  return useSyncExternalStore(subscribeUserCard, getUserCardState, getUserCardState);
}

/** The pointer came onto a trigger. Only a mouse hovers; touch and pen wait for the tap. */
export function hoverEnter(target: CardTarget, pointerType: string): void {
  if (pointerType !== "mouse") {
    return;
  }

  const current = state;

  // A card someone clicked open stays as it is until they dismiss it.
  if (shown(current) && current.via === "click") {
    return;
  }

  if (shown(current) && current.userId === target.userId) {
    clearTimer();
    set({ kind: "open", via: "hover", ...target });

    return;
  }

  if (current.kind === "pending" && current.userId === target.userId) {
    // Same person, another trigger: keep the delay already running, hang it off this one.
    set({ kind: "pending", ...target });

    return;
  }

  const warm = shown(current) || Date.now() - closedAt < WARM_WINDOW_MS;

  if (!shown(current)) {
    set({ kind: "pending", ...target });
  }

  // While a card shows, it keeps showing until the swap: no flash of nothing in between.
  schedule(warm ? WARM_DELAY_MS : OPEN_DELAY_MS, () => {
    set({ kind: "open", via: "hover", ...target });
  });
}

/** The pointer left a trigger. */
export function hoverLeave(): void {
  const current = state;

  if (current.kind === "pending") {
    clearTimer();
    set(CLOSED);

    return;
  }

  if (shown(current) && current.via === "hover") {
    set({ ...current, kind: "closing" });
    schedule(CLOSE_DELAY_MS, () => set(CLOSED));
  }
}

/** The pointer reached the card: it stays, and a swap someone else's trigger started is off. */
export function cardEnter(): void {
  const current = state;

  if (!shown(current)) {
    return;
  }

  clearTimer();
  set({ ...current, kind: "open" });
}

/** The pointer left the card. */
export function cardLeave(): void {
  hoverLeave();
}

/**
 * A click or tap on a trigger (or Enter or Space on one): the card opens at once and stays until
 * dismissed. Clicking the trigger of the card someone clicked open closes it again.
 */
export function openByClick(target: CardTarget): void {
  const current = state;

  clearTimer();

  if (
    current.kind === "open" &&
    current.via === "click" &&
    current.userId === target.userId &&
    current.anchor === target.anchor
  ) {
    set(CLOSED);

    return;
  }

  set({ kind: "open", via: "click", ...target });
}

/** Closes the card at once (Esc, an outside press, navigation, a scroll). */
export function closeUserCard(): void {
  clearTimer();
  set(CLOSED);
}

/**
 * A trigger or a body of message HTML is going away: a card hanging off it (or anything inside
 * it) closes, as it would otherwise float over nothing.
 */
export function releaseAnchor(element: Element): void {
  if (state.kind !== "closed" && element.contains(state.anchor)) {
    closeUserCard();
  }
}

/** Back to closed and cold, with nothing scheduled (tests). */
export function resetUserCard(): void {
  clearTimer();
  set(CLOSED);
  closedAt = Number.NEGATIVE_INFINITY;
}
