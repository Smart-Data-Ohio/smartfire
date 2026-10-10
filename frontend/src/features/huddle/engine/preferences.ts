/**
 * The huddle preferences this browser remembers. The keys are the classic app's, so a device
 * chosen there is still chosen here. Storage can be refused (private browsing): reads fall back to
 * the defaults and writes only last for the page.
 */
import type { StreamQuality } from "../../../gen/StreamQuality.ts";
import { DEFAULT_SHARE_QUALITY, knownStreamQuality } from "./screen-quality.ts";
import type { DeviceKind, ViewerQuality } from "./transport.ts";

export const DEVICE_STORAGE_KEY = "campfire.huddle.devices";

export const NOISE_SUPPRESSION_STORAGE_KEY = "campfire.huddle.noiseSuppression";

export const STREAM_QUALITY_STORAGE_KEY = "campfire.huddle.streamQuality";

export const SHARE_QUALITY_STORAGE_KEY = "campfire.huddle.shareQuality";

export const PARTICIPANT_VOLUME_PREFIX = "campfire.huddle.volume.";

export const PARTICIPANT_MUTE_PREFIX = "campfire.huddle.localMute.";

const KINDS: readonly DeviceKind[] = ["audioinput", "audiooutput", "videoinput"];

export type DevicePreferences = Readonly<Record<DeviceKind, string>>;

function read(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function write(key: string, value: string | null): void {
  try {
    if (value === null) {
      localStorage.removeItem(key);
    } else {
      localStorage.setItem(key, value);
    }
  } catch {
    // The preference only lasts for this page.
  }
}

/** The remembered microphone, speaker and camera ("" for the browser default). */
export function loadDevicePreferences(): DevicePreferences {
  const preferences: Record<DeviceKind, string> = {
    audioinput: "",
    audiooutput: "",
    videoinput: "",
  };

  try {
    const parsed: unknown = JSON.parse(read(DEVICE_STORAGE_KEY) ?? "{}");

    if (parsed instanceof Object) {
      const fields = new Map(Object.entries(parsed));

      for (const kind of KINDS) {
        const value = fields.get(kind);

        // Only strings are kept: anything else in the entry reads as no preference.
        if (`${value}` === value) {
          preferences[kind] = value;
        }
      }
    }
  } catch {
    // A corrupt entry reads as no preference.
  }

  return preferences;
}

/** Only ever written from the person's own choice, never from an SDK fallback. */
export function storeDevicePreference(kind: DeviceKind, deviceId: string): void {
  write(DEVICE_STORAGE_KEY, JSON.stringify({ ...loadDevicePreferences(), [kind]: deviceId }));
}

/** RNNoise is on unless the person turned it off. */
export function loadNoiseSuppression(): boolean {
  return read(NOISE_SUPPRESSION_STORAGE_KEY) !== "off";
}

export function storeNoiseSuppression(enabled: boolean): void {
  write(NOISE_SUPPRESSION_STORAGE_KEY, enabled ? "on" : "off");
}

export function loadStreamQuality(): ViewerQuality {
  const value = read(STREAM_QUALITY_STORAGE_KEY);

  return value === "low" || value === "high" ? value : "auto";
}

export function storeStreamQuality(quality: ViewerQuality): void {
  write(STREAM_QUALITY_STORAGE_KEY, quality);
}

/** The quality an ordinary screen share captures and encodes at; 1080p15 until picked. */
export function loadShareQuality(): StreamQuality {
  const value = read(SHARE_QUALITY_STORAGE_KEY);

  return knownStreamQuality(value) ?? DEFAULT_SHARE_QUALITY;
}

export function storeShareQuality(quality: StreamQuality): void {
  write(SHARE_QUALITY_STORAGE_KEY, quality);
}

/** Someone's volume for this browser, 0–200; 100 when unset (an empty entry isn't silence). */
export function loadParticipantVolume(userId: number): number {
  const raw = read(`${PARTICIPANT_VOLUME_PREFIX}${userId}`);

  if (raw === null || raw === "") {
    return 100;
  }

  const value = Number(raw);

  return Number.isFinite(value) ? Math.max(0, Math.min(200, value)) : 100;
}

export function storeParticipantVolume(userId: number, volume: number): void {
  write(`${PARTICIPANT_VOLUME_PREFIX}${userId}`, String(volume));
}

/** Whether the viewer muted someone for themselves. */
export function loadParticipantMuted(userId: number): boolean {
  return read(`${PARTICIPANT_MUTE_PREFIX}${userId}`) === "1";
}

export function storeParticipantMuted(userId: number, muted: boolean): void {
  write(`${PARTICIPANT_MUTE_PREFIX}${userId}`, muted ? "1" : null);
}
