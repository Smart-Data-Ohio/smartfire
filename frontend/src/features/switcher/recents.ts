/**
 * The switcher's recent picks (item keys, newest first), kept per browser so an empty ⌘K opens
 * on the places you actually go.
 */
const RECENTS_KEY = "smartfire.switcher.recents";

const MAX_RECENTS = 12;

export function readRecents(): readonly string[] {
  try {
    const saved = localStorage.getItem(RECENTS_KEY);

    return saved === null ? [] : saved.split(",").filter((key) => key !== "");
  } catch {
    return [];
  }
}

/** `key` moved to the front of `recents`, capped. */
export function withRecent(recents: readonly string[], key: string): readonly string[] {
  return [key, ...recents.filter((held) => held !== key)].slice(0, MAX_RECENTS);
}

export function recordRecent(key: string): void {
  try {
    localStorage.setItem(RECENTS_KEY, withRecent(readRecents(), key).join(","));
  } catch {
    // Unavailable storage only means the switcher forgets; nothing breaks.
  }
}
