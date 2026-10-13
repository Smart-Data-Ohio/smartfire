/** Per-conversation drafts in sessionStorage: a room's composer and each thread's keep their own. */

const DRAFT_PREFIX = "smartfire.draft.";

/**
 * How many times each draft's stored text has changed in this tab. A send that lands late
 * compares the revision it took with the current one: if someone has edited the draft since
 * — in this composer or in a later mount of the same conversation's — the send must not
 * clear it. Rewriting the same text (a remount putting its draft back) is not a change.
 */
const revisions = new Map<string, number>();

/** The last text written per key, so a tab without storage still tells a rewrite from an edit. */
const lastWritten = new Map<string, string>();

/** `12` for a room's composer, `12.t88` for a thread's. */
export function draftKey(roomId: number, threadId: number | null): string {
  return `${DRAFT_PREFIX}${roomId}${threadId === null ? "" : `.t${threadId}`}`;
}

/** `12.p501` for the first reply of a thread being started on message 501. */
export function newThreadDraftKey(roomId: number, parentMessageId: number): string {
  return `${DRAFT_PREFIX}${roomId}.p${parentMessageId}`;
}

/** The stored text, or `null` where storage itself is unavailable. */
function storedDraft(key: string): string | null {
  try {
    return sessionStorage.getItem(key) ?? "";
  } catch {
    return null;
  }
}

export function readDraft(key: string): string {
  return storedDraft(key) ?? "";
}

/** The draft's revision now: capture it as a send goes out, compare it as the send lands. */
export function draftRevision(key: string): number {
  return revisions.get(key) ?? 0;
}

export function writeDraft(key: string, text: string): void {
  const previous = storedDraft(key) ?? lastWritten.get(key) ?? "";

  lastWritten.set(key, text);

  if (previous !== text) {
    revisions.set(key, draftRevision(key) + 1);
  }

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

/**
 * Whether a draft is a typed command's fields: its text is then their line (`entryText`), and the
 * composer opens it as fields again rather than as text.
 */
export function readFieldsDraft(key: string): boolean {
  return readDraft(`${key}.fields`) !== "";
}

export function writeFieldsDraft(key: string, fields: boolean): void {
  writeDraft(`${key}.fields`, fields ? "1" : "");
}
