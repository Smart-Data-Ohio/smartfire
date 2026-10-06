/**
 * The call's sounds, as the classic app makes them: the join blip (`incoming.mp3`, through the
 * same gates as every chat sound), the two-tone ring of an incoming huddle (440 + 480 Hz for a
 * second, every three seconds), and the hosts' raised-hand chime (660 then 880 Hz). The tones
 * are synthesized, so there is no asset to load. Autoplay policies suspend audio until the first
 * gesture: a ring waits for one instead of failing (the banner shows meanwhile).
 */
import type { Me } from "../../gen/Me.ts";
import incomingUrl from "./sounds/incoming.mp3";

function minutesInZone(now: Date, zone: string): number | null {
  try {
    const parts = new Intl.DateTimeFormat("en-US", {
      timeZone: zone,
      hour: "numeric",
      minute: "numeric",
      hourCycle: "h23",
    }).formatToParts(now);

    const hour = Number(parts.find((part) => part.type === "hour")?.value);
    const minute = Number(parts.find((part) => part.type === "minute")?.value);

    return Number.isFinite(hour) && Number.isFinite(minute) ? hour * 60 + minute : null;
  } catch {
    return null;
  }
}

/**
 * Whether chat sounds are off right now: do-not-disturb, quiet hours (an empty window never
 * mutes; overnight windows wrap past midnight) or an out-of-office that silences notifications.
 */
export function soundsMuted(me: Me | null, now: Date): boolean {
  if (me === null) {
    return false;
  }

  const dnd = me.doNotDisturb;

  if (dnd.enabled && (dnd.until === null || Date.parse(dnd.until) > now.getTime())) {
    return true;
  }

  const away = me.outOfOffice;

  if (away !== null && !away.keepNotifications && Date.parse(away.until) > now.getTime()) {
    return true;
  }

  const quiet = me.quietHours;

  if (quiet === null || quiet.startMinute === quiet.endMinute) {
    return false;
  }

  const zone = me.preferences.timeZone ?? Intl.DateTimeFormat().resolvedOptions().timeZone;
  const minute = minutesInZone(now, zone);

  if (minute === null) {
    return false;
  }

  return quiet.startMinute < quiet.endMinute
    ? minute >= quiet.startMinute && minute < quiet.endMinute
    : minute >= quiet.startMinute || minute < quiet.endMinute;
}

/** The blip for someone joining the viewer's call. */
export function playJoinSound(): void {
  try {
    void new Audio(incomingUrl).play().catch(() => undefined);
  } catch {
    // No audio element support: the toast still says it.
  }
}

let context: AudioContext | null = null;

function audioContext(): AudioContext | null {
  if (globalThis.AudioContext === undefined) {
    return null;
  }

  try {
    context ??= new AudioContext();
  } catch {
    return null;
  }

  return context;
}

/** One envelope-shaped sine tone into `destination`. */
function tone(
  audio: AudioContext,
  destination: AudioNode,
  frequency: number,
  at: number,
  length: number,
  peak: number,
): void {
  const oscillator = audio.createOscillator();
  const envelope = audio.createGain();

  oscillator.type = "sine";
  oscillator.frequency.value = frequency;
  envelope.gain.setValueAtTime(0, at);
  envelope.gain.linearRampToValueAtTime(peak, at + Math.min(0.05, length / 6));
  envelope.gain.setValueAtTime(peak, at + length * 0.9);
  envelope.gain.linearRampToValueAtTime(0, at + length);
  oscillator.connect(envelope);
  envelope.connect(destination);
  oscillator.start(at);
  oscillator.stop(at + length + 0.05);
}

/** The raised-hand chime for hosts. */
export function playHandChime(): void {
  const audio = audioContext();

  if (audio === null) {
    return;
  }

  const play = () => {
    const at = audio.currentTime;

    tone(audio, audio.destination, 660, at, 0.12, 0.12);
    tone(audio, audio.destination, 880, at + 0.12, 0.12, 0.12);
  };

  if (audio.state === "suspended") {
    void audio
      .resume()
      .then(play)
      .catch(() => undefined);

    return;
  }

  play();
}

const RING_EVERY_MS = 3000;

/** An incoming call's ring: `start` until `stop`, waiting for a gesture if audio is locked. */
export class Ringer {
  #wanted = false;
  #timer: ReturnType<typeof setInterval> | null = null;
  #gain: GainNode | null = null;
  #unlock: (() => void) | null = null;

  start(): void {
    this.#wanted = true;

    if (this.#unlock === null) {
      this.#unlock = () => this.#ensure(false);
      window.addEventListener("pointerdown", this.#unlock);
      window.addEventListener("keydown", this.#unlock);
    }

    this.#ensure(false);
  }

  stop(): void {
    this.#wanted = false;

    if (this.#timer !== null) {
      clearInterval(this.#timer);
      this.#timer = null;
    }

    try {
      this.#gain?.disconnect();
    } catch {
      // Already gone; stopping the interval is what matters.
    }

    this.#gain = null;
    this.#dropUnlock();
  }

  #dropUnlock(): void {
    if (this.#unlock !== null) {
      window.removeEventListener("pointerdown", this.#unlock);
      window.removeEventListener("keydown", this.#unlock);
      this.#unlock = null;
    }
  }

  #ensure(retried: boolean): void {
    if (!this.#wanted || this.#timer !== null) {
      return;
    }

    const audio = audioContext();

    if (audio === null) {
      return;
    }

    if (audio.state === "suspended") {
      // One chained retry per gesture; the unlock listeners keep trying on later ones.
      if (!retried) {
        void audio
          .resume()
          .then(() => this.#ensure(true))
          .catch(() => undefined);
      }

      return;
    }

    const gain = audio.createGain();

    gain.gain.value = 0.12;
    gain.connect(audio.destination);

    const ring = () => {
      if (!this.#wanted) {
        return;
      }

      const at = audio.currentTime;

      tone(audio, gain, 440, at, 1, 1);
      tone(audio, gain, 480, at, 1, 1);
    };

    ring();
    this.#gain = gain;
    this.#timer = setInterval(ring, RING_EVERY_MS);
    this.#dropUnlock();
  }
}
