/** Drive picker decisions, shared by the composer panel and its tests. */

export const DRIVE_SEARCH_DEBOUNCE_MS = 300;

export const DRIVE_FILES_PER_MESSAGE = 10;

/** The first search runs at once; later keystrokes wait, as the classic picker does. */
export function searchDelay(subsequent: boolean): number {
  return subsequent ? DRIVE_SEARCH_DEBOUNCE_MS : 0;
}

/** Moves the highlighted row, wrapping at both ends. An empty list stays at -1. */
export function moveActive(index: number, delta: number, count: number): number {
  if (count === 0) {
    return -1;
  }

  if (index < 0) {
    return delta < 0 ? count - 1 : 0;
  }

  return (index + delta + count) % count;
}

export type PickerKey = "close" | "next" | "previous" | "choose";

/** Escape closes, arrows move, Enter chooses. Anything else is left to the field. */
export function pickerKeyAction(key: string): PickerKey | null {
  switch (key) {
    case "Escape":
      return "close";
    case "ArrowDown":
      return "next";
    case "ArrowUp":
      return "previous";
    case "Enter":
      return "choose";
    default:
      return null;
  }
}

export interface DrivePick {
  readonly id: string;
  readonly name: string;
  readonly kind: string;
  readonly url: string | null;
}

export type AttachResult =
  | { readonly status: "attached"; readonly files: readonly DrivePick[] }
  | { readonly status: "duplicate" | "full"; readonly files: readonly DrivePick[] };

/** Pins a file on the pending message. A duplicate is a no-op; the tenth is the last. */
export function attachDriveFile(files: readonly DrivePick[], file: DrivePick): AttachResult {
  if (files.some((current) => current.id === file.id)) {
    return { status: "duplicate", files };
  }

  if (files.length >= DRIVE_FILES_PER_MESSAGE) {
    return { status: "full", files };
  }

  return { status: "attached", files: [...files, file] };
}

/** Grant stays off until a person is checked, and folders are never shared. */
export function grantEnabled(selected: number, kind: string): boolean {
  return selected > 0 && kind !== "folder";
}

/** A 404 from search is "not connected", the same empty miss classic returns. */
export function driveDisconnected(error: {
  readonly tag: string;
  readonly message: string;
}): boolean {
  return error.tag === "NotFound" || error.message.includes("404");
}

/** Status line for a finished search. Disconnected is its own panel, not this string. */
export function driveSearchStatus(
  error: { readonly tag: string; readonly message: string } | null,
  count: number,
): string {
  if (error === null) {
    return count === 0 ? "No files found" : "";
  }

  if (driveDisconnected(error)) {
    return "";
  }

  if (error.message.includes("429") || error.message.includes("rate_limited")) {
    return "Try again in a moment";
  }

  return "Drive is unavailable right now";
}
