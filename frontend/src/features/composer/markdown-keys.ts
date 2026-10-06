/** A textarea's text and selection. */
export interface TextEdit {
  readonly value: string;
  readonly start: number;
  readonly end: number;
}

/**
 * Wraps the selection in a Markdown marker (`**`, `_`, `~~`, `` ` ``), or unwraps it when it's
 * already wrapped. With nothing selected it inserts the pair and puts the caret between them.
 */
export function toggleWrap(edit: TextEdit, marker: string): TextEdit {
  const { value, start, end } = edit;
  const before = value.slice(0, start);
  const selected = value.slice(start, end);
  const after = value.slice(end);
  const size = marker.length;

  if (before.endsWith(marker) && after.startsWith(marker)) {
    return {
      value: before.slice(0, -size) + selected + after.slice(size),
      start: start - size,
      end: end - size,
    };
  }

  if (selected.length >= size * 2 && selected.startsWith(marker) && selected.endsWith(marker)) {
    const inner = selected.slice(size, -size);

    return { value: before + inner + after, start, end: start + inner.length };
  }

  return {
    value: before + marker + selected + marker + after,
    start: start + size,
    end: end + size,
  };
}

/**
 * Makes the selection a Markdown link (`[text](url)`) with the url placeholder selected, so the
 * next paste or keystroke fills it. A selected URL becomes the target instead.
 */
export function insertLink(edit: TextEdit): TextEdit {
  const { value, start, end } = edit;
  const selected = value.slice(start, end);
  const isUrl = /^https?:\/\/\S+$/.test(selected);
  const text = isUrl ? "" : selected;
  const url = isUrl ? selected : "url";
  const link = `[${text}](${url})`;
  const next = value.slice(0, start) + link + value.slice(end);

  if (isUrl || text === "") {
    // Caret inside the brackets, ready for the label.
    return { value: next, start: start + 1, end: start + 1 };
  }

  const urlStart = start + text.length + 3;

  return { value: next, start: urlStart, end: urlStart + url.length };
}

/** The formatting chords: ⌘/Ctrl + B, I, E, ⇧X, ⇧U (Slack's link chord; ⌘K stays the switcher). */
export function markerForChord(key: string, shift: boolean): string | "link" | null {
  const lower = key.toLowerCase();

  if (!shift && lower === "b") {
    return "**";
  }

  if (!shift && lower === "i") {
    return "_";
  }

  if (!shift && lower === "e") {
    return "`";
  }

  if (shift && lower === "x") {
    return "~~";
  }

  if (shift && lower === "u") {
    return "link";
  }

  return null;
}
