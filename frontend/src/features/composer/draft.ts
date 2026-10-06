/** Per-conversation drafts in sessionStorage: a room's composer and each thread's keep their own. */

const DRAFT_PREFIX = "smartfire.draft.";

/** `12` for a room's composer, `12.t88` for a thread's. */
export function draftKey(roomId: number, threadId: number | null): string {
  return `${DRAFT_PREFIX}${roomId}${threadId === null ? "" : `.t${threadId}`}`;
}

/** `12.p501` for the first reply of a thread being started on message 501. */
export function newThreadDraftKey(roomId: number, parentMessageId: number): string {
  return `${DRAFT_PREFIX}${roomId}.p${parentMessageId}`;
}

export function readDraft(key: string): string {
  try {
    return sessionStorage.getItem(key) ?? "";
  } catch {
    return "";
  }
}

export function writeDraft(key: string, text: string): void {
  try {
    if (text === "") {
      sessionStorage.removeItem(key);
    } else {
      sessionStorage.setItem(key, text);
    }
  } catch {
    // Without storage a draft only lives while the room is open.
  }
}
