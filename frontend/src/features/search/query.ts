/**
 * The client's reading of a search query, mirroring the server's parser (`SearchQuery::parse`)
 * as far as the UI needs it: which words to highlight in a hit, and which operator the person is
 * typing so the typeahead can finish it. The server stays the authority on what matched; its
 * chips are what the filter bar shows.
 */
import type { SearchOperator } from "../../gen/SearchOperator.ts";

/** Every operator, in the order the typeahead offers them. */
export const OPERATORS: readonly SearchOperator[] = [
  "from",
  "in",
  "has",
  "is",
  "before",
  "after",
  "on",
];

const OPERATOR_TOKEN = /(?:^|[ \t\n\v\f\r])((?:from|in|has|before|after|on|is):[^ \t\n\v\f\r]+)/gu;

const WORD = /[\p{L}\p{N}\p{M}_]+/gu;

/** The query's free-text words (operators left out), as the server splits them. */
export function textWords(query: string): string[] {
  const text = query.replace(OPERATOR_TOKEN, " ");

  return [...text.matchAll(WORD)].map((match) => match[0]);
}

/** The operator the person is typing at the end of `value`, and what they've typed of its value. */
export interface PartialOperator {
  readonly operator: SearchOperator;
  readonly partial: string;
}

/** The last whitespace-separated piece of `value` (empty after a trailing space). */
export function lastToken(value: string): string {
  const match = /[^ \t\n\v\f\r]*$/u.exec(value);

  return match?.[0] ?? "";
}

/** `value`'s last token, read as an operator being typed (`from:ma`), or `null`. */
export function partialOperator(value: string): PartialOperator | null {
  const match = /^(from|in|has|before|after|on|is):(.*)$/u.exec(lastToken(value));
  const operator = OPERATORS.find((candidate) => candidate === match?.[1]);

  return operator === undefined ? null : { operator, partial: match?.[2] ?? "" };
}

/** The operators whose names start with `value`'s last token (two letters at least). */
export function operatorPrefixes(value: string): SearchOperator[] {
  const token = lastToken(value).toLowerCase();

  if (token.length < 2 || token.includes(":")) {
    return [];
  }

  return OPERATORS.filter((operator) => operator.startsWith(token));
}

/** `value` with its last token replaced by `token` and a space after, ready for the next word. */
export function replaceLastToken(value: string, token: string): string {
  return `${value.slice(0, value.length - lastToken(value).length)}${token} `;
}

/** `query` with `token` added at the end (a filter pill), separated by one space. */
export function appendToken(query: string, token: string): string {
  const trimmed = query.trimEnd();

  return trimmed === "" ? token : `${trimmed} ${token}`;
}

/** A local calendar day as an operator value: `2026-10-06`. */
export function isoDay(millis: number): string {
  const date = new Date(millis);
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");

  return `${date.getFullYear()}-${month}-${day}`;
}
