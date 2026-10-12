/**
 * Soundboard plays. Playing a sound asks the server (`POST /rooms/:id/huddle/soundboard`), which
 * checks the caller is in the call and may speak there, then sends `huddle.soundboard` to
 * everyone in the call, the caller's own tabs included. Each tab in that call plays the sound
 * itself, on its call speaker, at the sound's default volume times its own soundboard volume:
 * nothing is published into the call, so the microphone path is untouched. LiveKit data messages
 * would need a token permission the gateway refuses, hence the relay.
 *
 * A tab still checks every play it receives: live (not replayed after a reconnect) and recent,
 * for the call it is in, from someone it sees in that call and doesn't ignore, of a sound it
 * knows (refetching the catalog once for a sound it doesn't), with soundboard sounds on and the
 * call not deafened, and within the limits: one play per second per person, three at once.
 */
import type { HuddleSoundboardPlayed } from "../../gen/HuddleSoundboardPlayed.ts";
import type { SoundboardCatalog } from "../../gen/SoundboardCatalog.ts";
import type { SoundboardSound } from "../../gen/SoundboardSound.ts";
import { serverNow } from "../../store/server-clock.ts";
import { store } from "../../store/store.ts";
import { onSoundboardPlay, soundboard } from "../../sync/huddles.ts";
import { ActionError } from "../../sync/run.ts";
import { toast } from "../../ui/toast-store.ts";
import { callController } from "./call-controller.ts";
import { callStore, livePhase, userIdForIdentity } from "./call-store.ts";
import { heldSound, loadSoundboardCatalog, soundboardStore } from "./soundboard-store.ts";

/** The least time between two plays from one person, here and on the play button. */
export const SENDER_INTERVAL_MS = 1_000;

/** How many sounds may play at once. */
export const MAX_OVERLAPPING = 3;

/** A play older than this (by the server's clock) is dropped: it would land out of context. */
export const STALE_AFTER_MS = 10_000;

/** A sound's loudness here, 0–1: its default volume (0–1) times this browser's volume (0–100). */
export function clipVolume(soundVolume: number, localVolume: number): number {
  const sound = Number.isFinite(soundVolume) ? Math.max(0, Math.min(1, soundVolume)) : 1;
  const local = Number.isFinite(localVolume) ? Math.max(0, Math.min(100, localVolume)) : 100;

  return sound * (local / 100);
}

/** Spaces each person's plays and caps the sounds playing at once. */
export class PlayLimiter {
  readonly #last = new Map<number, number>();
  readonly #interval: number;
  readonly #overlap: number;
  #playing = 0;

  constructor(interval = SENDER_INTERVAL_MS, overlap = MAX_OVERLAPPING) {
    this.#interval = interval;
    this.#overlap = overlap;
  }

  /** Whether `sender`'s play at `now` (ms) may start; one that may counts as playing. */
  admit(sender: number, now: number): boolean {
    const last = this.#last.get(sender);

    if (this.#playing >= this.#overlap || (last !== undefined && now - last < this.#interval)) {
      return false;
    }

    this.#last.set(sender, now);
    this.#playing += 1;

    return true;
  }

  /** An admitted play ended. */
  finished(): void {
    this.#playing = Math.max(0, this.#playing - 1);
  }

  get playing(): number {
    return this.#playing;
  }
}

/** Why a received play didn't play. */
export type SkipReason =
  | "replayed"
  | "stale"
  | "other-call"
  | "off"
  | "deafened"
  | "not-in-call"
  | "ignored"
  | "unknown-sound"
  | "limited";

/** What a tab knows when a play arrives. */
export interface PlayContext {
  /** The room of the call this tab is connected to, `null` outside one. */
  readonly callRoomId: number | null;
  /** The LiveKit identities in this tab's call, its own included. */
  readonly identities: ReadonlySet<string>;
  /** The person behind an identity, from the room's presence. */
  readonly userFor: (identity: string) => number | null;
  readonly ignored: (userId: number) => boolean;
  readonly enabled: boolean;
  readonly deafened: boolean;
  /** The server's time now (ms), `null` until known. */
  readonly serverNow: number | null;
}

/** The first reason a play must not play here, before its sound is looked up; `null` if none. */
export function screenPlay(
  event: HuddleSoundboardPlayed,
  live: boolean,
  context: PlayContext,
): SkipReason | null {
  if (!live) {
    return "replayed";
  }

  const playedAt = Date.parse(event.playedAt);

  if (
    context.serverNow !== null &&
    Number.isFinite(playedAt) &&
    context.serverNow - playedAt > STALE_AFTER_MS
  ) {
    return "stale";
  }

  if (context.callRoomId !== event.roomId) {
    return "other-call";
  }

  if (!context.enabled) {
    return "off";
  }

  if (context.deafened) {
    return "deafened";
  }

  if (!context.identities.has(event.identity) || context.userFor(event.identity) !== event.userId) {
    return "not-in-call";
  }

  return context.ignored(event.userId) ? "ignored" : null;
}

/** Everything the player reaches outside itself, so tests can stand in for it. */
export interface PlayerDeps {
  readonly context: () => PlayContext;
  /** A held sound, `undefined` when not held. */
  readonly sound: (soundId: number) => SoundboardSound | undefined;
  /** Refetches the catalog. */
  readonly refresh: () => Promise<SoundboardCatalog>;
  /** This browser's soundboard volume, 0–100. */
  readonly volume: () => number;
  /** Plays on the call's speaker; `null` when the call is gone. */
  readonly play: (roomId: number, url: string, volume: number) => Promise<void> | null;
  /** A monotonic clock (ms). */
  readonly now: () => number;
}

export interface SoundboardPlayer {
  /** Handles one relayed play; answers why it was skipped, or `null` once it started. */
  received(event: HuddleSoundboardPlayed, live: boolean): Promise<SkipReason | null>;
}

export function createSoundboardPlayer(deps: PlayerDeps): SoundboardPlayer {
  const limiter = new PlayLimiter();
  /** Sounds refetched for once already: an id still unknown after a fetch that worked is skipped. */
  const refetched = new Set<number>();

  const lookUp = async (soundId: number): Promise<SoundboardSound | undefined> => {
    const held = deps.sound(soundId);

    if (held !== undefined || refetched.has(soundId)) {
      return held;
    }

    refetched.add(soundId);

    try {
      await deps.refresh();
    } catch {
      // A failed fetch says nothing about the sound: the next play of it asks again.
      refetched.delete(soundId);

      return undefined;
    }

    return deps.sound(soundId);
  };

  return {
    async received(event, live) {
      const early = screenPlay(event, live, deps.context());

      if (early !== null) {
        return early;
      }

      const sound = await lookUp(event.soundId);

      if (sound === undefined) {
        return "unknown-sound";
      }

      // The call or the preferences may have moved while the catalog loaded.
      const late = screenPlay(event, true, deps.context());

      if (late !== null) {
        return late;
      }

      if (!limiter.admit(event.userId, deps.now())) {
        return "limited";
      }

      const playing = deps.play(
        event.roomId,
        sound.audioUrl,
        clipVolume(sound.volume, deps.volume()),
      );

      if (playing === null) {
        limiter.finished();

        return "other-call";
      }

      void playing.finally(() => limiter.finished());

      return null;
    },
  };
}

/** The player behind the app's calls. */
export const soundboardPlayer = createSoundboardPlayer({
  context: () => {
    const call = callStore.getState();
    const state = store.getState();
    const roomId = livePhase(call.phase) ? call.roomId : null;
    const presence = roomId === null ? undefined : state.huddles[roomId];
    const preferences = soundboardStore.getState();
    const clock = state.sidebar.serverClock;

    return {
      callRoomId: roomId,
      identities: new Set(call.snapshot.participants.map((participant) => participant.identity)),
      userFor: (identity) => userIdForIdentity(presence, identity),
      ignored: (userId) => state.ignoredPeople[userId]?.ignoredByMe === true,
      enabled: preferences.enabled,
      deafened: call.deafened,
      serverNow: clock === undefined ? null : serverNow(clock),
    };
  },
  sound: heldSound,
  refresh: () => loadSoundboardCatalog(true),
  volume: () => soundboardStore.getState().volume,
  play: (roomId, url, volume) => callController.playClip(roomId, url, volume),
  now: () => performance.now(),
});

/** Plays every relayed play as it arrives; returns the unsubscribe. */
export function listenForSoundboard(): () => void {
  return onSoundboardPlay((play, live) => {
    void soundboardPlayer.received(play, live);
  });
}

let lastSent = Number.NEGATIVE_INFINITY;

/**
 * Plays `soundId` to everyone in the viewer's call. Answers false when nothing was sent: no call,
 * sounds turned off, or another play less than a second ago. A refusal says why in a toast.
 */
export function playToCall(soundId: number): boolean {
  const call = callStore.getState();
  const now = performance.now();

  if (
    !livePhase(call.phase) ||
    call.roomId === null ||
    call.identity === null ||
    !call.canPublish ||
    !soundboardStore.getState().enabled ||
    now - lastSent < SENDER_INTERVAL_MS
  ) {
    return false;
  }

  lastSent = now;

  soundboard.play(call.roomId, call.identity, soundId).catch((error: Error) => {
    const slow = error instanceof ActionError && error.tag === "RateLimited";

    toast({
      title: slow ? "Too many sounds" : "Couldn’t play that sound",
      description: slow ? "Wait a moment before playing another." : error.message,
      tone: "danger",
    });
  });

  return true;
}

/** Forgets the last play's time (tests). */
export function resetPlayCooldown(): void {
  lastSent = Number.NEGATIVE_INFINITY;
}
