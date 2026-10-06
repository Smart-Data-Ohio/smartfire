/**
 * Which autocomplete the caret is in, from the text and the caret alone: `@` people, `:` emoji
 * and icons, `/` commands (only as the very first thing in the message) and `#` rooms. A trigger
 * counts only at the start of a line or after whitespace or `(`, so `ada@example.com`, `12:30`
 * and `issue#4` never open a menu.
 */

export type TriggerKind = "mention" | "emoji" | "command" | "room";

export interface Trigger {
  readonly kind: TriggerKind;
  /** What's typed after the trigger character, up to the caret. */
  readonly query: string;
  /** Where the trigger character sits. */
  readonly start: number;
  /** The caret: the completion replaces `start..end`. */
  readonly end: number;
}

/** Whether the character before `index` lets a trigger start there. */
function boundaryBefore(text: string, index: number): boolean {
  if (index === 0) {
    return true;
  }

  return /[\s(]/.test(text.charAt(index - 1));
}

const MENTION_QUERY = /^(?! )[^\n@[\]]{0,50}$/;

const EMOJI_QUERY = /^[a-z0-9_+-]{2,40}$/i;

const ROOM_QUERY = /^[^\s#[\]()]{0,80}$/;

const COMMAND_TEXT = /^\/([a-z0-9_-]*)$/i;

/** Up to three words: "Ada", "Ada Love", "Mary Ann Smi". */
function mentionWords(query: string): boolean {
  return query.split(" ").length <= 3;
}

function candidate(
  line: string,
  lineStart: number,
  caret: number,
  character: string,
  kind: TriggerKind,
  accepts: (query: string) => boolean,
): Trigger | null {
  const index = line.lastIndexOf(character);

  if (index < 0 || !boundaryBefore(line, index)) {
    return null;
  }

  const query = line.slice(index + 1);

  return accepts(query) ? { kind, query, start: lineStart + index, end: caret } : null;
}

/** The trigger the caret is in, or `null`. With a selection, pass its end and get `null`. */
export function findTrigger(text: string, caret: number, selectionEnd = caret): Trigger | null {
  if (selectionEnd !== caret) {
    return null;
  }

  const before = text.slice(0, caret);
  const command = COMMAND_TEXT.exec(before);

  if (command !== null) {
    return { kind: "command", query: command[1] ?? "", start: 0, end: caret };
  }

  const lineStart = before.lastIndexOf("\n") + 1;
  const line = before.slice(lineStart);

  const found = [
    candidate(
      line,
      lineStart,
      caret,
      "@",
      "mention",
      (query) => MENTION_QUERY.test(query) && mentionWords(query),
    ),
    candidate(line, lineStart, caret, ":", "emoji", (query) => EMOJI_QUERY.test(query)),
    candidate(line, lineStart, caret, "#", "room", (query) => ROOM_QUERY.test(query)),
  ];

  let nearest: Trigger | null = null;

  for (const trigger of found) {
    if (trigger !== null && (nearest === null || trigger.start > nearest.start)) {
      nearest = trigger;
    }
  }

  return nearest;
}

/** The text after accepting a completion, and where the caret goes. */
export interface Completion {
  readonly value: string;
  readonly caret: number;
}

/**
 * Replaces the trigger and its query with `insertion`, then one space unless the text after
 * already starts with whitespace (or `trailingSpace` is off).
 */
export function applyCompletion(
  text: string,
  trigger: Trigger,
  insertion: string,
  trailingSpace = true,
): Completion {
  const after = text.slice(trigger.end);
  const space = trailingSpace && !/^\s/.test(after) ? " " : "";
  const value = text.slice(0, trigger.start) + insertion + space + after;

  return { value, caret: trigger.start + insertion.length + (trailingSpace ? 1 : 0) };
}

/** An in-app link to a room: `[#name](/rooms/12)` (the contract has no room-reference syntax). */
export function roomLink(name: string, roomId: number): string {
  const label = name.replace(/[[\]]/g, "");

  return `[#${label}](/rooms/${roomId})`;
}
