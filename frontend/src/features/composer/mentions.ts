/** A picked mention keeps its displayed text and the server token it must send. */
import type { TextEdit } from "./markdown-keys.ts";
import type { DraftMention } from "./mentions-schema.ts";

export type Mention = DraftMention;

export interface MentionText {
  readonly text: string;
  readonly mentions: readonly Mention[];
}

/** Only recorded, still-matching mentions become tokens. Typed @names stay literal. */
export function mentionMarkdown({ text, mentions }: MentionText): string {
  let markdown = "";
  let cursor = 0;

  for (const mention of mentions) {
    if (
      mention.start < cursor ||
      text.slice(mention.start, mention.start + mention.text.length) !== mention.text
    ) {
      continue;
    }

    markdown += text.slice(cursor, mention.start) + mention.token;
    cursor = mention.start + mention.text.length;
  }

  return markdown + text.slice(cursor);
}

/** Hydrates stable tokens; escaped tokens and Markdown code remain literal source. */
export function hydrateMentions(
  markdown: string,
  nameOf: (id: number) => string | undefined,
): MentionText {
  const mentions: Mention[] = [];
  // Backtick runs include fenced blocks and multi-backtick spans. Tilde fences also stay code.
  const parts = /(?<code>`+|~{3,})[\s\S]*?\k<code>(?![`~])|<@(?<id>[1-9][0-9]*)>/g;
  let text = "";
  let cursor = 0;

  for (const match of markdown.matchAll(parts)) {
    const id = match.groups?.id;

    const name =
      id === undefined || markdown[match.index - 1] === "\\" ? undefined : nameOf(Number(id));

    if (name === undefined) {
      continue;
    }

    text += markdown.slice(cursor, match.index);
    const label = `@${name}`;

    mentions.push({ start: text.length, text: label, token: match[0] });
    text += label;
    cursor = match.index + match[0].length;
  }

  return { text: text + markdown.slice(cursor), mentions };
}

export interface MentionEdit {
  readonly start: number;
  readonly end: number;
}

function changedRange(before: string, after: string): MentionEdit {
  let start = 0;

  while (start < before.length && start < after.length && before[start] === after[start]) {
    start += 1;
  }

  let end = before.length;
  let nextEnd = after.length;

  while (end > start && nextEnd > start && before[end - 1] === after[nextEnd - 1]) {
    end -= 1;
    nextEnd -= 1;
  }

  return { start, end };
}

/** Pass the replaced selection for textarea edits: identical names make text-only diffs ambiguous. */
export function editMentions(
  current: MentionText,
  text: string,
  { start, end }: MentionEdit = changedRange(current.text, text),
): MentionText {
  const delta = text.length - current.text.length;

  const mentions = current.mentions.flatMap((mention) => {
    const mentionEnd = mention.start + mention.text.length;

    if (mentionEnd <= start) {
      if (mentionEnd === start && /^[\p{L}\p{N}\p{M}_]/u.test(text.slice(mentionEnd))) return [];

      return [mention];
    }

    if (mention.start >= end) {
      const nextStart = mention.start + delta;

      if (mention.start === end && /[\p{L}\p{N}\p{M}_]$/u.test(text.slice(0, nextStart))) return [];

      return [{ ...mention, start: nextStart }];
    }

    return [];
  });

  return { text, mentions };
}

/** Emphasis wraps keep identities; code and links intentionally become literal names. */
export function formatMentions(current: MentionText, edit: TextEdit): MentionText {
  for (const marker of ["**", "*", "__", "_", "~~"]) {
    const size = marker.length;
    const start = edit.start - size;
    const end = edit.end - size;

    if (
      start >= 0 &&
      edit.value ===
        current.text.slice(0, start) +
          marker +
          current.text.slice(start, end) +
          marker +
          current.text.slice(end)
    ) {
      return {
        text: edit.value,
        mentions: current.mentions.flatMap((mention) => {
          const mentionEnd = mention.start + mention.text.length;

          if (mentionEnd <= start) return [mention];

          if (mention.start >= end) return [{ ...mention, start: mention.start + 2 * size }];

          if (mention.start >= start && mentionEnd <= end) {
            return [{ ...mention, start: mention.start + size }];
          }

          return [];
        }),
      };
    }

    // toggleWrap also removes markers, with or without the markers in the selection.
    if (
      current.text ===
      edit.value.slice(0, edit.start) +
        marker +
        edit.value.slice(edit.start, edit.end) +
        marker +
        edit.value.slice(edit.end)
    ) {
      return {
        text: edit.value,
        mentions: current.mentions.flatMap((mention) => {
          const mentionEnd = mention.start + mention.text.length;

          if (mentionEnd <= edit.start) return [mention];

          if (mention.start >= edit.end + 2 * size) {
            return [{ ...mention, start: mention.start - 2 * size }];
          }

          if (mention.start >= edit.start + size && mentionEnd <= edit.end + size) {
            return [{ ...mention, start: mention.start - size }];
          }

          return [];
        }),
      };
    }
  }

  return editMentions(current, edit.value);
}
