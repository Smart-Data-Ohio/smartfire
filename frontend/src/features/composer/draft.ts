/** Per-conversation drafts in sessionStorage: a room's composer and each thread's keep their own. */

import { decodeDraftMentions } from "../../sync/composer-actions.ts";
import type { Mention, MentionText } from "./mentions.ts";

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

/** Old text-only drafts have no ranges; their tokens are hydrated by the composer. */
export function readMentionDraft(key: string): MentionText {
  const text = readDraft(key);
  const mentions: Mention[] = [];

  try {
    const saved = decodeDraftMentions(sessionStorage.getItem(`${key}.mentions`) ?? "[]");
    let end = 0;

    for (const item of saved) {
      if (
        item.start < end ||
        !item.text.startsWith("@") ||
        !/^<@[1-9][0-9]*>$/.test(item.token) ||
        text.slice(item.start, item.start + item.text.length) !== item.text
      ) {
        continue;
      }

      mentions.push(item);
      end = item.start + item.text.length;
    }
  } catch {
    // Unavailable storage or stale metadata leaves the draft as plain text.
  }

  return { text, mentions };
}

export function writeDraft(key: string, text: string, mentions: readonly Mention[] = []): void {
  const metadata = mentions.length === 0 ? "" : JSON.stringify(mentions);
  const previous = storedDraft(key) ?? lastWritten.get(key) ?? "";

  const previousMetadata =
    storedDraft(`${key}.mentions`) ?? lastWritten.get(`${key}.mentions`) ?? "";

  lastWritten.set(key, text);
  lastWritten.set(`${key}.mentions`, metadata);

  if (previous !== text || previousMetadata !== metadata) {
    revisions.set(key, draftRevision(key) + 1);
  }

  try {
    if (metadata === "") {
      sessionStorage.removeItem(`${key}.mentions`);
    } else {
      sessionStorage.setItem(`${key}.mentions`, metadata);
    }

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
