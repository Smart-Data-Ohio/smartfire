/**
 * Call notices, as the classic `huddle_join_notice_controller.js` shows them: in the viewer's
 * call, a toast ("Chris and Dean joined", batched over four seconds with one blip; "Dean left",
 * at most three); for a call elsewhere, a banner in that room ("Chris is in your huddle", with
 * Join) and a pill under its sidebar row. A server mute's revoke-and-rejoin must read as nothing
 * at all, so a leave waits five seconds and a join from the same person inside that window
 * cancels both; a join the server marks `rejoin` arms a one-shot quiet window for a leave still
 * in flight, unless that leave already toasted. Presence is authoritative for banners: a call
 * that empties clears its banner, and people no longer in it drop from the list.
 */
import { createStore } from "zustand/vanilla";
import type { HuddleNotice } from "../../gen/HuddleNotice.ts";

export const JOIN_TOAST_MS = 6000;

export const JOIN_BATCH_MS = 4000;

export const LEAVE_DELAY_MS = 5000;

export const LEAVE_TOAST_MS = 4000;

const MAX_LEAVE_TOASTS = 3;

export interface Joiner {
  readonly id: number;
  readonly name: string;
}

export interface NoticeBanner {
  readonly roomId: number;
  readonly roomName: string;
  readonly joiners: readonly Joiner[];
}

export interface CallToast {
  readonly id: number;
  readonly kind: "join" | "leave";
  readonly text: string;
}

export interface NoticeState {
  readonly banners: Readonly<Record<number, NoticeBanner>>;
  readonly toasts: readonly CallToast[];
}

export const noticeStore = createStore<NoticeState>()(() => ({ banners: {}, toasts: [] }));

/** "Chris", "Chris and Dean", "Chris, Dean, and Erin". */
export function sentence(names: readonly string[]): string {
  if (names.length > 2) {
    return `${names.slice(0, -1).join(", ")}, and ${names.at(-1)}`;
  }

  return names.join(" and ");
}

export function bannerText(joiners: readonly Joiner[]): string {
  const names = joiners.map((joiner) => joiner.name);

  if (names.length > 1) {
    return `${sentence(names)} are in your huddle`;
  }

  if (names.length === 1) {
    return `${names[0]} is in your huddle`;
  }

  return "Someone is in your huddle";
}

export interface NoticeEnvironment {
  /** The viewer is in (or joining) this room's call in this tab. */
  inCall(roomId: number): boolean;
  playJoinSound(): void;
}

type Timer = ReturnType<typeof setTimeout>;

function key(roomId: number, userId: number): string {
  return `${roomId}:${userId}`;
}

interface JoinBatch {
  readonly roomId: number;
  readonly names: string[];
  readonly startedAt: number;
  readonly toastId: number;
}

export class CallNotices {
  readonly #env: NoticeEnvironment;
  readonly #pendingLeaves = new Map<string, Timer>();
  readonly #recentRejoins = new Map<string, Timer>();
  readonly #firedLeaves = new Map<string, { readonly at: number; readonly timer: Timer }>();
  /** Each leave toast's dismissal, by toast id. */
  readonly #toastTimers = new Map<number, Timer>();
  #batch: JoinBatch | null = null;
  #batchTimer: Timer | null = null;
  #nextToast = 1;

  constructor(env: NoticeEnvironment) {
    this.#env = env;
  }

  received(notice: HuddleNotice): void {
    switch (notice.kind) {
      case "joined":
        this.#joined(notice);

        return;
      case "left":
        // The other half of a mute cycle whose join already stayed quiet: the joiner never left.
        if (this.#consumeRejoin(notice.roomId, notice.userId)) {
          return;
        }

        if (this.#env.inCall(notice.roomId)) {
          this.#scheduleLeaveToast(notice.roomId, notice.userId, notice.userName);
        }

        this.#removeJoiner(notice.roomId, notice.userId);

        return;
      case "ended":
        this.dismiss(notice.roomId);

        return;
    }
  }

  /** The viewer's call went active in a room: joining answers its banner. */
  callActive(roomId: number): void {
    this.dismiss(roomId);
  }

  /** A room's presence changed: an empty call clears its banner; absent people drop from it. */
  presenceChanged(roomId: number, userIds: readonly number[]): void {
    const banner = noticeStore.getState().banners[roomId];

    if (banner === undefined) {
      return;
    }

    const present = new Set(userIds);

    this.#setJoiners(
      banner,
      banner.joiners.filter((joiner) => present.has(joiner.id)),
    );
  }

  dismiss(roomId: number): void {
    const banners = noticeStore.getState().banners;

    if (banners[roomId] === undefined) {
      return;
    }

    const next = { ...banners };

    delete next[roomId];
    noticeStore.setState({ banners: next });
  }

  dismissToast(id: number): void {
    if (this.#batch?.toastId === id) {
      this.#endBatch();
    }

    this.#removeToast(id);
  }

  /** Drops every timer (sign-out, tests). Banners and toasts clear too. */
  reset(): void {
    for (const timer of [...this.#pendingLeaves.values(), ...this.#recentRejoins.values()]) {
      clearTimeout(timer);
    }

    for (const fired of this.#firedLeaves.values()) {
      clearTimeout(fired.timer);
    }

    for (const timer of this.#toastTimers.values()) {
      clearTimeout(timer);
    }

    this.#toastTimers.clear();
    this.#pendingLeaves.clear();
    this.#recentRejoins.clear();
    this.#firedLeaves.clear();
    this.#endBatch();
    noticeStore.setState({ banners: {}, toasts: [] });
  }

  // ── Joins ─────────────────────────────────────────────────────────────────────────────────

  #joined(notice: Extract<HuddleNotice, { kind: "joined" }>): void {
    const { roomId, userId } = notice;

    // A join inside the leave delay answers that leave and stays quiet itself.
    if (this.#cancelPendingLeave(roomId, userId)) {
      return;
    }

    let alreadyLeft = false;

    if (notice.rejoin && this.#leaveInFlight(roomId, userId)) {
      if (this.#consumeFiredLeave(roomId, userId)) {
        alreadyLeft = true;
      } else {
        this.#armRejoin(roomId, userId);
      }
    }

    if (!this.#env.inCall(roomId)) {
      this.#addJoiner(notice);

      return;
    }

    if (!notice.rejoin || alreadyLeft) {
      this.#clearFiredLeave(roomId, userId);
      this.#toastJoin(roomId, notice.userName);
    }
  }

  /** A marked rejoin's leave is still in flight while the viewer still shows them present. */
  #leaveInFlight(roomId: number, userId: number): boolean {
    if (this.#env.inCall(roomId)) {
      return true;
    }

    const banner = noticeStore.getState().banners[roomId];

    return banner?.joiners.some((joiner) => joiner.id === userId) === true;
  }

  #toastJoin(roomId: number, name: string): void {
    if (name === "") {
      return;
    }

    const now = Date.now();
    const batch = this.#batch;

    if (batch !== null && batch.roomId === roomId && now - batch.startedAt < JOIN_BATCH_MS) {
      if (!batch.names.includes(name)) {
        batch.names.push(name);
      }

      this.#updateToast(batch.toastId, `${sentence(batch.names)} joined`);
    } else {
      this.#endBatch();

      const toastId = this.#addToast("join", `${name} joined`);

      this.#batch = { roomId, names: [name], startedAt: now, toastId };
      this.#env.playJoinSound();
    }

    const toastId = this.#batch?.toastId;

    if (this.#batchTimer !== null) {
      clearTimeout(this.#batchTimer);
    }

    this.#batchTimer = setTimeout(() => {
      this.#batchTimer = null;
      this.#batch = null;

      if (toastId !== undefined) {
        this.#removeToast(toastId);
      }
    }, JOIN_TOAST_MS);
  }

  #endBatch(): void {
    if (this.#batchTimer !== null) {
      clearTimeout(this.#batchTimer);
      this.#batchTimer = null;
    }

    this.#batch = null;
  }

  // ── Leaves ────────────────────────────────────────────────────────────────────────────────

  #scheduleLeaveToast(roomId: number, userId: number, name: string): void {
    if (name === "") {
      return;
    }

    this.#cancelPendingLeave(roomId, userId);
    this.#clearFiredLeave(roomId, userId);

    const timer = setTimeout(() => {
      this.#pendingLeaves.delete(key(roomId, userId));

      if (this.#env.inCall(roomId)) {
        this.#showLeaveToast(name);
        this.#recordFiredLeave(roomId, userId);
      }
    }, LEAVE_DELAY_MS);

    this.#pendingLeaves.set(key(roomId, userId), timer);
  }

  #showLeaveToast(name: string): void {
    const id = this.#addToast("leave", `${name} left`);
    const leaves = noticeStore.getState().toasts.filter((toast) => toast.kind === "leave");

    for (const old of leaves.slice(0, Math.max(0, leaves.length - MAX_LEAVE_TOASTS))) {
      this.#removeToast(old.id);
    }

    this.#toastTimers.set(
      id,
      setTimeout(() => this.#removeToast(id), LEAVE_TOAST_MS),
    );
  }

  #cancelPendingLeave(roomId: number, userId: number): boolean {
    const timer = this.#pendingLeaves.get(key(roomId, userId));

    if (timer === undefined) {
      return false;
    }

    clearTimeout(timer);
    this.#pendingLeaves.delete(key(roomId, userId));

    return true;
  }

  #armRejoin(roomId: number, userId: number): void {
    this.#consumeRejoin(roomId, userId);
    this.#recentRejoins.set(
      key(roomId, userId),
      setTimeout(() => this.#recentRejoins.delete(key(roomId, userId)), LEAVE_DELAY_MS),
    );
  }

  #consumeRejoin(roomId: number, userId: number): boolean {
    const timer = this.#recentRejoins.get(key(roomId, userId));

    if (timer === undefined) {
      return false;
    }

    clearTimeout(timer);
    this.#recentRejoins.delete(key(roomId, userId));

    return true;
  }

  #recordFiredLeave(roomId: number, userId: number): void {
    this.#clearFiredLeave(roomId, userId);
    this.#firedLeaves.set(key(roomId, userId), {
      at: Date.now(),
      timer: setTimeout(() => this.#firedLeaves.delete(key(roomId, userId)), LEAVE_DELAY_MS),
    });
  }

  #consumeFiredLeave(roomId: number, userId: number): boolean {
    const fired = this.#firedLeaves.get(key(roomId, userId));

    if (fired === undefined) {
      return false;
    }

    clearTimeout(fired.timer);
    this.#firedLeaves.delete(key(roomId, userId));

    return Date.now() - fired.at <= LEAVE_DELAY_MS;
  }

  #clearFiredLeave(roomId: number, userId: number): void {
    const fired = this.#firedLeaves.get(key(roomId, userId));

    if (fired !== undefined) {
      clearTimeout(fired.timer);
      this.#firedLeaves.delete(key(roomId, userId));
    }
  }

  // ── Banners and toasts ────────────────────────────────────────────────────────────────────

  #addJoiner(notice: Extract<HuddleNotice, { kind: "joined" }>): void {
    const banners = noticeStore.getState().banners;
    const held = banners[notice.roomId];
    const joiners = held?.joiners ?? [];

    const next: NoticeBanner = {
      roomId: notice.roomId,
      roomName: notice.roomName === "" ? (held?.roomName ?? "") : notice.roomName,
      joiners: joiners.some((joiner) => joiner.id === notice.userId)
        ? joiners
        : [...joiners, { id: notice.userId, name: notice.userName }],
    };

    noticeStore.setState({ banners: { ...banners, [notice.roomId]: next } });
  }

  #removeJoiner(roomId: number, userId: number): void {
    const banner = noticeStore.getState().banners[roomId];

    if (banner !== undefined) {
      this.#setJoiners(
        banner,
        banner.joiners.filter((joiner) => joiner.id !== userId),
      );
    }
  }

  /** The last name out hides the banner. */
  #setJoiners(banner: NoticeBanner, joiners: readonly Joiner[]): void {
    if (joiners.length === banner.joiners.length) {
      return;
    }

    if (joiners.length === 0) {
      this.dismiss(banner.roomId);

      return;
    }

    noticeStore.setState({
      banners: { ...noticeStore.getState().banners, [banner.roomId]: { ...banner, joiners } },
    });
  }

  #addToast(kind: CallToast["kind"], text: string): number {
    const id = this.#nextToast;

    this.#nextToast += 1;
    noticeStore.setState({ toasts: [...noticeStore.getState().toasts, { id, kind, text }] });

    return id;
  }

  #updateToast(id: number, text: string): void {
    noticeStore.setState({
      toasts: noticeStore
        .getState()
        .toasts.map((toast) => (toast.id === id ? { ...toast, text } : toast)),
    });
  }

  #removeToast(id: number): void {
    const timer = this.#toastTimers.get(id);

    if (timer !== undefined) {
      clearTimeout(timer);
      this.#toastTimers.delete(id);
    }

    const toasts = noticeStore.getState().toasts;

    if (toasts.some((toast) => toast.id === id)) {
      noticeStore.setState({ toasts: toasts.filter((toast) => toast.id !== id) });
    }
  }
}
