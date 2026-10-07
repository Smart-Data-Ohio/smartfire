/**
 * An incoming huddle, as the classic `huddle_invitation_controller.js` rings it: a banner
 * ("Maya started a huddle", Join and Dismiss) and a ring unless the server says `silent`, plus a
 * system notification when the tab is hidden and permission was already granted. It never rings
 * for a call the viewer is in. A ring stops by itself after 45 seconds (the server's missed-call
 * wait); "ended" for the ring on screen turns it into "Maya left the huddle" for five seconds;
 * any other news about the same inbox item (answered in another tab) hides it. Joining marks the
 * item handled and dismissing marks it read, when there is an item.
 */
import { createStore } from "zustand/vanilla";
import type { HuddleRing } from "../../gen/HuddleRing.ts";

export const RING_TIMEOUT_MS = 45_000;

export const ENDED_TIMEOUT_MS = 5000;

export interface RingState {
  readonly ring: HuddleRing | null;
  /** `ringing` shows Join and Dismiss; `ended` says the caller left. */
  readonly phase: "ringing" | "ended";
}

export const ringStore = createStore<RingState>()(() => ({ ring: null, phase: "ringing" }));

export interface RingEnvironment {
  /** The viewer is in (or joining) this room's call in this tab. */
  inCall(roomId: number): boolean;
  startRinging(): void;
  stopRinging(): void;
  /** A system notification for a hidden tab; returns its closer, or `null` if none showed. */
  notify(ring: HuddleRing): (() => void) | null;
  answer(activityItemId: number, action: "handled" | "read"): void;
  /** Goes to the room and joins its call. */
  join(roomId: number, roomName: string): void;
}

export class IncomingCalls {
  readonly #env: RingEnvironment;
  #timer: ReturnType<typeof setTimeout> | null = null;
  #closeNotification: (() => void) | null = null;

  constructor(env: RingEnvironment) {
    this.#env = env;
  }

  received(ring: HuddleRing): void {
    const current = ringStore.getState();

    if (ring.event === "started" && ring.state === "unread") {
      this.#show(ring);
    } else if (ring.event === "ended" && this.#matches(ring)) {
      this.#showEnded(ring);
    } else if (
      current.ring !== null &&
      ring.activityItemId !== null &&
      ring.activityItemId === current.ring.activityItemId
    ) {
      this.hide();
    }
  }

  /** The viewer's call went active in a room: a ring for it is answered. */
  callActive(roomId: number): void {
    if (ringStore.getState().ring?.roomId === roomId) {
      this.hide();
    }
  }

  join(): void {
    const ring = ringStore.getState().ring;

    this.hide();

    if (ring === null) {
      return;
    }

    if (ring.activityItemId !== null) {
      this.#env.answer(ring.activityItemId, "handled");
    }

    this.#env.join(ring.roomId, ring.roomName);
  }

  dismiss(): void {
    const ring = ringStore.getState().ring;

    this.hide();

    if (ring?.activityItemId !== null && ring?.activityItemId !== undefined) {
      this.#env.answer(ring.activityItemId, "read");
    }
  }

  hide(): void {
    this.#clearTimer();
    this.#env.stopRinging();
    this.#closeNotification?.();
    this.#closeNotification = null;
    ringStore.setState({ ring: null, phase: "ringing" });
  }

  #show(ring: HuddleRing): void {
    if (this.#env.inCall(ring.roomId)) {
      return;
    }

    ringStore.setState({ ring, phase: "ringing" });
    this.#closeNotification?.();
    this.#closeNotification = null;

    if (ring.silent) {
      this.#env.stopRinging();
    } else {
      this.#env.startRinging();
      this.#closeNotification = this.#env.notify(ring);
    }

    this.#clearTimer();
    this.#timer = setTimeout(() => this.hide(), RING_TIMEOUT_MS);
  }

  #showEnded(ring: HuddleRing): void {
    this.#clearTimer();
    this.#env.stopRinging();
    this.#closeNotification?.();
    this.#closeNotification = null;

    const shown = ringStore.getState().ring;

    ringStore.setState({
      ring: { ...ring, activityItemId: ring.activityItemId ?? shown?.activityItemId ?? null },
      phase: "ended",
    });
    this.#timer = setTimeout(() => this.hide(), ENDED_TIMEOUT_MS);
  }

  /** By inbox item when there is one, by room for rings without an item. */
  #matches(ring: HuddleRing): boolean {
    const shown = ringStore.getState().ring;

    if (shown === null) {
      return false;
    }

    if (ring.activityItemId !== null) {
      return ring.activityItemId === shown.activityItemId;
    }

    return ring.roomId === shown.roomId;
  }

  #clearTimer(): void {
    if (this.#timer !== null) {
      clearTimeout(this.#timer);
      this.#timer = null;
    }
  }
}
