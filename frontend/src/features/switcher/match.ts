/**
 * Fuzzy matching for the quick switcher and the people pickers: a query matches a label when
 * its letters appear in order (a subsequence), scored so an exact name beats a prefix, a prefix
 * beats a word start, and tight runs at word boundaries beat letters scattered across the name.
 */

/** Lowercased, accents folded, a leading `#` or `@` dropped (people type them out of habit). */
export function normalizeQuery(query: string): string {
  return fold(query.trim().replace(/^[#@]+/, ""));
}

export function fold(text: string): string {
  return text
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase();
}

const BOUNDARY = /[\s\-_./,&]/;

function atWordStart(text: string, index: number): boolean {
  return index === 0 || BOUNDARY.test(text[index - 1] ?? "");
}

/**
 * The subsequence score: rewards runs and word starts, charges for gaps; `null` if no match. A
 * query of one or two letters only matches as initials ("jl" finds Jonah Lindqvist), since two
 * letters scattered anywhere would match most names.
 */
function subsequenceScore(label: string, query: string): number | null {
  if (query.length <= 2) {
    return initialsScore(label, query);
  }

  let score = 0;
  let from = 0;
  let previous = -2;

  for (const char of query) {
    const index = label.indexOf(char, from);

    if (index === -1) {
      return null;
    }

    if (index === previous + 1) {
      score += 6;
    } else if (atWordStart(label, index)) {
      score += 8;
    } else {
      score -= Math.min(index - from, 6);
    }

    previous = index;
    from = index + 1;
  }

  return score;
}

/** Matches `query` against the label's word initials only; `null` if they don't spell it. */
function initialsScore(label: string, query: string): number | null {
  let from = 0;

  for (const char of query) {
    let index = label.indexOf(char, from);

    while (index !== -1 && !atWordStart(label, index)) {
      index = label.indexOf(char, index + 1);
    }

    if (index === -1) {
      return null;
    }

    from = index + 1;
  }

  return query.length * 8;
}

/**
 * Where `query` (already normalized) appears as one run in `label`, accents folded, as
 * `[start, end)` in the label's own indices; `null` when it doesn't, or when folding would shift
 * the indices (a letter that folds to two).
 */
export function matchRange(label: string, query: string): readonly [number, number] | null {
  if (query === "") {
    return null;
  }

  const pieces = [...label];
  const folded = pieces.map(fold);

  if (folded.some((piece, index) => piece.length !== (pieces[index] ?? "").length)) {
    return null;
  }

  const index = folded.join("").indexOf(query);

  return index === -1 ? null : [index, index + query.length];
}

/**
 * How well `query` (already normalized) matches `label`: higher is better, `null` is no match.
 * An empty query matches everything with 0.
 */
export function matchScore(label: string, query: string): number | null {
  if (query === "") {
    return 0;
  }

  const text = fold(label);

  if (text === query) {
    return 1000;
  }

  if (text.startsWith(query)) {
    return 800 - Math.min(text.length - query.length, 100);
  }

  const index = text.indexOf(query);

  if (index !== -1) {
    return (atWordStart(text, index) ? 600 : 400) - Math.min(index, 100);
  }

  const fuzzy = subsequenceScore(text, query);

  return fuzzy === null ? null : 200 + fuzzy;
}
