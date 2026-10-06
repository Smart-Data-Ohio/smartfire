import type { ReactNode } from "react";

/**
 * A preview of the Markdown a pending message was typed in, until the server's rendered HTML
 * replaces it a moment later. Only the inline marks the composer's shortcuts produce are drawn,
 * without nesting; everything else stays as typed. React builds the nodes, so nothing the user
 * typed is ever parsed as HTML.
 */
type Mark = "plain" | "code" | "strong" | "strike" | "em";

export interface InlineSegment {
  readonly mark: Mark;
  readonly text: string;
  /** Where the segment starts in the source: unique, so it doubles as the React key. */
  readonly start: number;
}

const INLINE = /`([^`\n]+)`|\*\*([^*\n]+)\*\*|~~([^~\n]+)~~|\*([^*\n]+)\*|_([^_\n]+)_/g;

const GROUP_MARKS = ["code", "strong", "strike", "em", "em"] as const satisfies readonly Mark[];

export function inlineSegments(source: string): InlineSegment[] {
  const segments: InlineSegment[] = [];
  let last = 0;

  for (const match of source.matchAll(INLINE)) {
    const groupIndex = match.slice(1).findIndex((group) => group !== undefined);
    const mark = GROUP_MARKS[groupIndex];
    const text = match[groupIndex + 1];

    if (mark === undefined || text === undefined) continue;

    if (match.index > last) {
      segments.push({ mark: "plain", text: source.slice(last, match.index), start: last });
    }

    segments.push({ mark, text, start: match.index });
    last = match.index + match[0].length;
  }

  if (last < source.length) segments.push({ mark: "plain", text: source.slice(last), start: last });

  return segments;
}

const RENDER: Record<Mark, (text: string) => ReactNode> = {
  plain: (text) => text,
  code: (text) => <code>{text}</code>,
  strong: (text) => <strong>{text}</strong>,
  strike: (text) => <s>{text}</s>,
  em: (text) => <em>{text}</em>,
};

export function InlineMarkdown({ source }: { readonly source: string }) {
  return inlineSegments(source).map((segment) => (
    <span key={segment.start}>{RENDER[segment.mark](segment.text)}</span>
  ));
}
