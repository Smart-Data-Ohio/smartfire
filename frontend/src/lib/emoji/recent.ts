import { useSyncExternalStore } from "react";

/** A reaction's content as it's posted: an emoji character, or `:name:` for an icon. */
export interface EmojiChoice {
  readonly content: string;
  /** The tooltip name ("Thumbs up"). */
  readonly title: string;
  /** Icons only: the image to draw. */
  readonly imageUrl: string | null;
  /** An icon's first frame, when known: the still to show under reduced motion. */
  readonly stillUrl?: string | null;
}

/** The server's quick reactions (`EmojiHelper::REACTIONS`), in order. */
export const QUICK_REACTIONS: readonly EmojiChoice[] = [
  { content: "👍", title: "Thumbs up", imageUrl: null },
  { content: "❤️", title: "Red heart", imageUrl: null },
  { content: "😂", title: "Face with tears of joy", imageUrl: null },
  { content: "🎉", title: "Party popper", imageUrl: null },
  { content: "👏", title: "Clapping", imageUrl: null },
  { content: "🔥", title: "Fire", imageUrl: null },
  { content: "👋", title: "Waving hand", imageUrl: null },
  { content: "💪", title: "Muscle", imageUrl: null },
];

const STORAGE_KEY = "smartfire:emoji-recent";

/** How many recent picks the picker remembers. */
export const RECENT_LIMIT = 27;

const EMPTY: readonly EmojiChoice[] = [];

const listeners = new Set<() => void>();

let cache: readonly EmojiChoice[] | null = null;

function isChoice(value: EmojiChoice | null): value is EmojiChoice {
  return (
    value !== null &&
    `${value.content}` === value.content &&
    value.content !== "" &&
    `${value.title}` === value.title
  );
}

function read(): readonly EmojiChoice[] {
  if (cache !== null) {
    return cache;
  }

  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    // SAFETY: parsed from our own key; every entry is checked by isChoice before use.
    const parsed = raw === null ? [] : (JSON.parse(raw) as readonly (EmojiChoice | null)[]);

    cache = Array.isArray(parsed)
      ? parsed.filter(isChoice).map((choice) => ({ ...choice, imageUrl: choice.imageUrl ?? null }))
      : EMPTY;
  } catch {
    cache = EMPTY;
  }

  return cache;
}

/** Moves `choice` to the front of the recent list (most recent first, no repeats). */
export function withRecent(
  list: readonly EmojiChoice[],
  choice: EmojiChoice,
  limit = RECENT_LIMIT,
): EmojiChoice[] {
  return [choice, ...list.filter((entry) => entry.content !== choice.content)].slice(0, limit);
}

/** Remembers a pick for the Recent tab and the quick reactions. */
export function recordRecentEmoji(choice: EmojiChoice): void {
  cache = withRecent(read(), choice);

  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(cache));
  } catch {
    // Private mode or a full quota: the list still holds for this session.
  }

  for (const listener of listeners) {
    listener();
  }
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);

  return () => listeners.delete(listener);
}

/** The recent picks, most recent first; live across components. */
export function useRecentEmoji(): readonly EmojiChoice[] {
  return useSyncExternalStore(subscribe, read, () => EMPTY);
}

/** `count` quick reactions: the most recent picks, topped up with the server's defaults. */
export function quickReactions(recent: readonly EmojiChoice[], count = 3): EmojiChoice[] {
  const picked: EmojiChoice[] = [];

  for (const choice of [...recent, ...QUICK_REACTIONS]) {
    if (picked.length === count) {
      break;
    }

    if (!picked.some((entry) => entry.content === choice.content)) {
      picked.push(choice);
    }
  }

  return picked;
}

/** Drops the remembered list (tests). */
export function resetRecentEmoji(): void {
  cache = null;
}
