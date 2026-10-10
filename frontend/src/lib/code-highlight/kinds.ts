/**
 * The kinds of coloured token a code block can hold. Each is a class (`code-token--<kind>`) whose
 * colour is a design token (`--code-<kind>` in tokens.css), so light, dark and every palette use
 * the app's own colours and nothing from a highlighter theme reaches the page.
 */
export const TOKEN_KINDS = [
  "comment",
  "keyword",
  "control",
  "string",
  "regexp",
  "special",
  "number",
  "variable",
  "constant",
  "function",
  "type",
  "punctuation",
  "invalid",
  "inserted",
  "deleted",
] as const;

export type TokenKind = (typeof TOKEN_KINDS)[number];

const KINDS: ReadonlySet<string> = new Set(TOKEN_KINDS);

export function isTokenKind(value: string): value is TokenKind {
  return KINDS.has(value);
}

/** One coloured run of a block's source: `source.slice(offset, offset + length)`. */
export interface CodeToken {
  readonly offset: number;
  readonly length: number;
  readonly kind: TokenKind;
}

/** A block's language (`"text"` when it stays plain) and its coloured runs, in source order. */
export interface HighlightResult {
  readonly language: string;
  readonly tokens: readonly CodeToken[];
}

export const PLAIN_RESULT: HighlightResult = { language: "text", tokens: [] };
