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

/** Grant stays off until a person is checked. Folders and shortcuts are attach-only. */
export function grantEnabled(selected: number, kind: string): boolean {
  return selected > 0 && kind !== "folder" && kind !== "shortcut";
}

/** A file already pinned still grants; an eleventh file does not, before any permission is written. */
export function grantCapacity(attachedIds: readonly string[], fileId: string): boolean {
  return attachedIds.includes(fileId) || attachedIds.length < DRIVE_FILES_PER_MESSAGE;
}

export const CONFIRMATION_MESSAGE =
  "Recipient details changed since this review. The list was refreshed with the changes unchecked; review it and confirm again.";

/** The notice for a share the server refused before granting. */
export function shareBlockedMessage(blocked: string): string {
  switch (blocked) {
    case "folder":
    case "shortcut":
      return "Folders and shortcuts cannot be shared from here. You can still attach the link; members open it with whatever access they already have.";
    case "capability":
      return "You do not have permission to share this file in Drive. You can still attach the link. To share it, change the sharing settings in Google Drive or ask the file owner; your organization's Drive policies may block sharing with people outside your organization.";
    case "full":
      return `Up to ${DRIVE_FILES_PER_MESSAGE} Drive files per message — remove one to grant and attach.`;
    default:
      return "Drive did not return this file's sharing status. You can still attach the link, or try again.";
  }
}

/** One recipient line, in the classic dialog's words. */
export function shareResultLabel(status: string, reason: string | null): string {
  if (status === "already") return "already had access";

  if (status === "granted") return "granted view access";

  switch (reason) {
    case "denied":
      return "not granted (refused by Google)";
    case "rate_limited":
      return "not granted (rate limited — retry shortly)";
    case "not_found":
      return "not granted (file unavailable in Drive)";
    case "unavailable":
      return "not granted (Google service error — retry shortly)";
    default:
      return "not granted (connection failed)";
  }
}

/** The summary above a partial or completed grant. Failures stay visible. */
export function shareSummary(results: readonly { readonly status: string }[]): string {
  const granted = results.filter((result) => result.status === "granted").length;
  const already = results.filter((result) => result.status === "already").length;
  const failed = results.filter((result) => result.status === "failed").length;
  const total = results.length;

  if (failed === 0 && already === 0) {
    return total === 1 ? "View access granted." : `View access granted to ${granted} recipients.`;
  }

  if (failed === 0 && granted === 0) return "Everyone selected already has access.";

  if (granted === 0 && already === 0) return "No access was granted.";

  return `${granted + already} of ${total} recipients have access.`;
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
