/**
 * Marks a hit's matching words. The contract carries no match ranges (the server's index stems
 * words with Porter), so the client approximates: a word in the body is marked when it starts
 * with a searched word or shares its rough stem ("invites" marks "invited"). Text inside code
 * blocks is marked too; tags and attributes never are.
 */
import { fold } from "../switcher/match.ts";

/** A light stem: the common English endings off, so a word's forms compare equal. */
export function roughStem(word: string): string {
  let lower = fold(word);

  if (lower.length > 4 && lower.endsWith("ies")) {
    lower = `${lower.slice(0, -3)}y`;
  } else if (lower.endsWith("sses")) {
    lower = lower.slice(0, -2);
  } else if (lower.length > 5 && lower.endsWith("ing")) {
    lower = lower.slice(0, -3);
  } else if (lower.length > 4 && lower.endsWith("ed")) {
    lower = lower.slice(0, -2);
  } else if (lower.length > 3 && lower.endsWith("s") && !lower.endsWith("ss")) {
    lower = lower.slice(0, -1);
  }

  return lower.length > 4 && lower.endsWith("e") ? lower.slice(0, -1) : lower;
}

/** A matcher for the searched words; `null` when there are none. */
export function wordMatcher(terms: readonly string[]): ((word: string) => boolean) | null {
  const folded = terms.flatMap((term) => {
    const value = fold(term);

    return value === "" ? [] : [value];
  });

  if (folded.length === 0) {
    return null;
  }

  const stems = new Set(folded.map(roughStem));

  return (word) => {
    const candidate = fold(word);

    return folded.some((term) => candidate.startsWith(term)) || stems.has(roughStem(word));
  };
}

const WORD = /[\p{L}\p{N}\p{M}_]+/gu;

/** `text` split into runs, each marked or not. Adjacent unmarked runs are merged. */
export function markRuns(
  text: string,
  matches: (word: string) => boolean,
): { readonly text: string; readonly marked: boolean }[] {
  const runs: { text: string; marked: boolean }[] = [];
  let from = 0;

  const push = (piece: string, marked: boolean) => {
    const last = runs.at(-1);

    if (piece === "") {
      return;
    }

    if (last !== undefined && !last.marked && !marked) {
      last.text += piece;

      return;
    }

    runs.push({ text: piece, marked });
  };

  for (const match of text.matchAll(WORD)) {
    const word = match[0];

    if (matches(word)) {
      push(text.slice(from, match.index), false);
      push(word, true);
      from = match.index + word.length;
    }
  }

  push(text.slice(from), false);

  return runs;
}

/**
 * `html` (the server's sanitized body) with the searched words wrapped in
 * `<mark class="search-mark">`. Unchanged when there's nothing to mark.
 */
export function highlightHtml(html: string, terms: readonly string[]): string {
  const matches = wordMatcher(terms);

  if (matches === null) {
    return html;
  }

  const template = document.createElement("template");

  template.innerHTML = html;

  const walker = document.createTreeWalker(template.content, NodeFilter.SHOW_TEXT);
  const texts: Text[] = [];

  for (let node = walker.nextNode(); node !== null; node = walker.nextNode()) {
    if (node instanceof Text) {
      texts.push(node);
    }
  }

  for (const node of texts) {
    // A match inside a spoiler would paint a highlight on the hidden words. Leave them covered.
    if (node.parentElement?.closest("[data-spoiler], .spoiler") != null) {
      continue;
    }

    const runs = markRuns(node.data, matches);

    if (runs.some((run) => run.marked)) {
      node.replaceWith(
        ...runs.map((run) => {
          if (!run.marked) {
            return run.text;
          }

          const mark = document.createElement("mark");

          mark.className = "search-mark";
          mark.textContent = run.text;

          return mark;
        }),
      );
    }
  }

  return template.innerHTML;
}
