/**
 * Device enumeration for the huddle's pickers. Enumerating never prompts; labels stay empty until
 * the page has microphone or camera access, so unlabeled devices get numbered names.
 */
import type { DeviceKind } from "./transport.ts";

export interface MediaDeviceOption {
  readonly deviceId: string;
  readonly label: string;
}

export type DeviceLists = Readonly<Record<DeviceKind, readonly MediaDeviceOption[]>>;

export const EMPTY_DEVICE_LISTS: DeviceLists = { audioinput: [], audiooutput: [], videoinput: [] };

const NAMES: Readonly<Record<DeviceKind, string>> = {
  audioinput: "Microphone",
  audiooutput: "Speaker",
  videoinput: "Camera",
};

/** What a picker says when there is nothing to pick. */
export const EMPTY_LABEL: Readonly<Record<DeviceKind, string>> = {
  audioinput: "No microphone found",
  audiooutput: "Default speaker",
  videoinput: "No camera found",
};

/** Every microphone, speaker and camera, grouped by kind, in the browser's order. */
export async function listDevices(): Promise<DeviceLists> {
  if (navigator.mediaDevices === undefined) {
    return EMPTY_DEVICE_LISTS;
  }

  const devices = await navigator.mediaDevices.enumerateDevices();

  const grouped: Record<DeviceKind, MediaDeviceOption[]> = {
    audioinput: [],
    audiooutput: [],
    videoinput: [],
  };

  for (const device of devices) {
    if (
      device.kind === "audioinput" ||
      device.kind === "audiooutput" ||
      device.kind === "videoinput"
    ) {
      const list = grouped[device.kind];

      list.push({
        deviceId: device.deviceId,
        label: device.label === "" ? `${NAMES[device.kind]} ${list.length + 1}` : device.label,
      });
    }
  }

  return grouped;
}

/** Speaker switching needs `setSinkId` (Safari has none): the picker hides without it. */
export function audioOutputSupported(): boolean {
  return "setSinkId" in HTMLMediaElement.prototype;
}

/** Whether any device has a label, i.e. the page was granted access before. */
export function devicesLabeled(lists: DeviceLists): boolean {
  return [...lists.audioinput, ...lists.videoinput].some(
    (device) => !/^(Microphone|Camera) \d+$/.test(device.label),
  );
}

/**
 * The first join in a browser stops at the device check; returning people skip it. A denied
 * permission skips it too, so the denial surfaces through the usual failure.
 */
export async function shouldCheckDevices(): Promise<boolean> {
  try {
    // SAFETY: "microphone" is a permission name Chromium, Firefox and Safari accept; lib.dom's
    // `PermissionName` lags behind. A browser that doesn't know it rejects, handled below.
    const status = await navigator.permissions.query({ name: "microphone" as PermissionName });

    return status.state === "prompt";
  } catch {
    // No Permissions API for the microphone: fall back to whether devices have labels.
  }

  try {
    const devices = await navigator.mediaDevices.enumerateDevices();

    return !devices.some((device) => device.label !== "");
  } catch {
    return true;
  }
}

/** Screen sharing needs getDisplayMedia; without it Share hides. */
export function canShareScreen(): boolean {
  return navigator.mediaDevices !== undefined && "getDisplayMedia" in navigator.mediaDevices;
}

/** A refused permission, as getUserMedia and getDisplayMedia report it. */
export function permissionDenied(error: Error): boolean {
  const message = error.message.toLowerCase();

  return (
    error.name === "NotAllowedError" || message.includes("permission") || message.includes("denied")
  );
}
