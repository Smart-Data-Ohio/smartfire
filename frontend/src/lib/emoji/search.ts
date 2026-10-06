import type { Emoji } from "./data.ts";

/** How well an emoji matches; lower is better, `null` is no match. */
function rank(emoji: Emoji, query: string): number | null {
  if (emoji.aliases.includes(query)) {
    return 0;
  }

  if (emoji.aliases.some((alias) => alias.startsWith(query))) {
    return 1;
  }

  const nameWords = emoji.name.split(" ");

  if (emoji.name.startsWith(query) || nameWords.some((word) => word.startsWith(query))) {
    return 2;
  }

  if (emoji.keywords.some((keyword) => keyword.startsWith(query))) {
    return 3;
  }

  if (emoji.aliases.some((alias) => alias.includes(query)) || emoji.name.includes(query)) {
    return 4;
  }

  return null;
}

/** "  :Thumbs_Up: " -> "thumbs_up": what people type, as the catalogue spells it. */
export function normalizeQuery(query: string): string {
  return query
    .trim()
    .toLowerCase()
    .replace(/^:+|:+$/g, "");
}

/**
 * Emoji matching `query`, best first: an exact shortcode, then shortcode prefixes, then words of
 * the name, then keywords, then any substring. Ties keep the catalogue's order. Every word of a
 * multi-word query has to match ("heart eyes").
 */
export function searchEmoji(all: readonly Emoji[], query: string, limit = 200): Emoji[] {
  const normalized = normalizeQuery(query);

  if (normalized === "") {
    return [];
  }

  const terms = normalized.split(/[\s_]+/).filter((term) => term !== "");
  const whole = normalized.replace(/\s+/g, "_");
  const scored: { emoji: Emoji; score: number; index: number }[] = [];

  all.forEach((emoji, index) => {
    const wholeRank = rank(emoji, whole);
    const termRanks = terms.map((term) => rank(emoji, term));

    if (wholeRank === null && termRanks.some((value) => value === null)) {
      return;
    }

    const termScore = termRanks.reduce<number>((sum, value) => sum + (value ?? 0), 0) + 1;

    scored.push({ emoji, score: wholeRank ?? termScore, index });
  });

  scored.sort((left, right) => left.score - right.score || left.index - right.index);

  return scored.slice(0, limit).map((entry) => entry.emoji);
}
