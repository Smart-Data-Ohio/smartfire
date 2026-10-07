/**
 * The call's own state, apart from the server's (who is in which call lives in the main store's
 * `huddles`). One call at a time per tab, as in the classic app; the controller writes this and
 * the dock, the call view and the launchers read it.
 */

import { useStore } from "zustand";
import { createStore } from "zustand/vanilla";
import type { HuddlePresence } from "../../gen/HuddlePresence.ts";
import type { StreamQuality } from "../../gen/StreamQuality.ts";
import { type DeviceLists, EMPTY_DEVICE_LISTS } from "./engine/devices.ts";
import type { DevicePreferences } from "./engine/preferences.ts";
import {
  type CallParticipant,
  type CallSnapshot,
  type ConnectionQuality,
  type ConnectionStats,
  EMPTY_SNAPSHOT,
} from "./engine/transport.ts";

/**
 * `prejoin` is the first-join device check; `failed` keeps the dock up with the reason and
 * Retry until the viewer closes it.
 */
export type CallPhase = "idle" | "prejoin" | "connecting" | "connected" | "reconnecting" | "failed";

/** The phases in which the viewer counts as being in (or joining) a call. */
export function activePhase(phase: CallPhase): boolean {
  return (
    phase === "prejoin" ||
    phase === "connecting" ||
    phase === "connected" ||
    phase === "reconnecting"
  );
}

/** In the room with media flowing (or about to again). */
export function livePhase(phase: CallPhase): boolean {
  return phase === "connected" || phase === "reconnecting";
}

export interface PrejoinState {
  readonly selected: DevicePreferences;
  readonly error: string | null;
  readonly canJoin: boolean;
  readonly retry: boolean;
  /** The camera preview, while one runs. */
  readonly preview: MediaStream | null;
}

export interface NoiseState {
  /** This browser can run RNNoise (latched off after an "unsupported" failure). */
  readonly available: boolean;
  readonly enabled: boolean;
  /** A switch is in progress. */
  readonly busy: boolean;
}

/** The stage stream this tab is presenting. */
export interface Streaming {
  readonly roomId: number;
  readonly quality: StreamQuality;
  readonly streamId: number;
}

export interface CallBusy {
  readonly microphone: boolean;
  readonly camera: boolean;
  readonly screen: boolean;
}

export interface CallState {
  readonly phase: CallPhase;
  readonly roomId: number | null;
  readonly roomName: string;
  /** The dock's status line ("Huddle active", "You're sharing your screen"…). */
  readonly status: string;
  /** Set while failed: "Couldn't join huddle" or "Huddle ended". */
  readonly failureTitle: string | null;
  /** A message that stays up (a failure, "A host muted you", a camera that wouldn't start). */
  readonly notice: string | null;
  /** The token can publish: false for stage listeners and the server-muted. */
  readonly canPublish: boolean;
  readonly identity: string | null;
  readonly snapshot: CallSnapshot;
  /** The microphone meter, 0–100 (the device check's preview too). */
  readonly meter: number;
  readonly quality: ConnectionQuality;
  /** Seconds left on the reconnect countdown; 0 shows "…"; `null` hides it. */
  readonly reconnectSeconds: number | null;
  readonly noise: NoiseState;
  /** Every remote voice silenced for this browser (and the microphone closed). */
  readonly deafened: boolean;
  readonly streaming: Streaming | null;
  readonly prejoin: PrejoinState | null;
  /** The in-call device pickers are open. */
  readonly devicesOpen: boolean;
  readonly devices: DeviceLists;
  /** The device each picker shows (the active one in a call, else the remembered one). */
  readonly selectedDevices: DevicePreferences;
  readonly busy: CallBusy;
  readonly statsOpen: boolean;
  readonly stats: ConnectionStats | null;
  /** The screen share shown large (theater mode), by video id. */
  readonly expandedVideoId: string | null;
  /** The call view is open over the room (rather than collapsed to the dock). */
  readonly viewOpen: boolean;
  /** When this room's call first connected (epoch ms), for the dock's timer; rejoins keep it. */
  readonly startedAt: number | null;
  /** The people in the call the viewer muted for themselves (remembered per person). */
  readonly localMutes: readonly number[];
}

export const NO_DEVICES: DevicePreferences = { audioinput: "", audiooutput: "", videoinput: "" };

export const initialCallState: CallState = {
  phase: "idle",
  roomId: null,
  roomName: "Huddle",
  status: "Not in a huddle",
  failureTitle: null,
  notice: null,
  canPublish: true,
  identity: null,
  snapshot: EMPTY_SNAPSHOT,
  meter: 0,
  quality: "unknown",
  reconnectSeconds: null,
  noise: { available: false, enabled: false, busy: false },
  deafened: false,
  streaming: null,
  prejoin: null,
  devicesOpen: false,
  devices: EMPTY_DEVICE_LISTS,
  selectedDevices: NO_DEVICES,
  busy: { microphone: false, camera: false, screen: false },
  statsOpen: false,
  stats: null,
  expandedVideoId: null,
  viewOpen: false,
  startedAt: null,
  localMutes: [],
};

export const callStore = createStore<CallState>()(() => initialCallState);

/** The person behind a LiveKit identity in a room's presence (one per tab, so maybe several). */
export function userIdForIdentity(
  presence: HuddlePresence | undefined,
  identity: string,
): number | null {
  for (const participant of presence?.participants ?? []) {
    if (participant.identities.includes(identity)) {
      return participant.userId;
    }
  }

  return null;
}

/**
 * The stage stream's screen share among the call's participants: this tab's own share while it
 * streams, otherwise the live presenter's (when they're in this call and sharing).
 */
export function streamVideoIdOf(
  participants: readonly CallParticipant[],
  streamingHere: boolean,
  presenter: string | null,
): string | null {
  if (streamingHere) {
    return participants.find((participant) => participant.local)?.screenId ?? null;
  }

  if (presenter === null) {
    return null;
  }

  return participants.find((participant) => participant.identity === presenter)?.screenId ?? null;
}

export function useCall<T>(selector: (state: CallState) => T): T {
  return useStore(callStore, selector);
}
