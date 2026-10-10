/**
 * Plain-text previews say "spoiler" instead of the hidden words. The renderer treats `||…||` as
 * one inline span that does not cross a blank line or a code span, and these helpers follow that.
 */

const FENCE = /^( {0,3})(`{3,}|~{3,})/;

function escaped(source: string, index: number): boolean {
  let slashes = 0;

  for (let cursor = index - 1; cursor >= 0 && source[cursor] === "\\"; cursor -= 1) {
    slashes += 1;
  }

  return slashes % 2 === 1;
}

function lineEnd(source: string, index: number): number {
  const newline = source.indexOf("\n", index);

  return newline === -1 ? source.length : newline + 1;
}

/** A fence opener at `index` (a line start), or null. `next` is the index just after that line. */
function openFence(
  source: string,
  index: number,
): { readonly mark: string; readonly next: number } | null {
  const line = FENCE.exec(source.slice(index));

  if (line === null) {
    return null;
  }

  const indent = line[1] ?? "";
  const mark = line[2] ?? "";

  return { mark, next: lineEnd(source, index + indent.length) };
}

function closesFence(source: string, index: number, mark: string): boolean {
  let cursor = index;
  let spaces = 0;

  while (spaces < 4 && source[cursor] === " ") {
    spaces += 1;
    cursor += 1;
  }

  if (!source.startsWith(mark, cursor)) {
    return false;
  }

  let end = cursor + mark.length;
  const char = mark[0];

  while (source[end] === char) {
    end += 1;
  }

  const newline = source.indexOf("\n", end);
  const rest = source.slice(end, newline === -1 ? undefined : newline);

  return rest.trim() === "";
}

/** The index after an inline code span that starts at `index`, or one past a lone unmatched tick. */
function endOfInlineCode(source: string, index: number): number {
  let length = 0;

  while (source[index + length] === "`") {
    length += 1;
  }

  const closer = "`".repeat(length);
  const found = source.indexOf(closer, index + length);

  return found === -1 ? index + 1 : found + length;
}

function blankBetween(source: string, start: number, end: number): boolean {
  return /\n[ \t]*\n/.test(source.slice(start, end));
}

/** `||hidden||` outside code becomes the word "spoiler". Unclosed pairs and code stay as written. */
export function redactMarkdownSpoilers(source: string): string {
  const hidden = Array.from({ length: source.length }, () => false);
  const stack: number[] = [];
  let index = 0;
  let fence: string | null = null;

  const lineStart = (at: number) => at === 0 || source[at - 1] === "\n";

  while (index < source.length) {
    if (fence !== null) {
      if (lineStart(index) && closesFence(source, index, fence)) {
        index = lineEnd(source, index);
        fence = null;
      } else {
        index += 1;
      }

      continue;
    }

    if (lineStart(index)) {
      const opened = openFence(source, index);

      if (opened !== null) {
        stack.length = 0;
        fence = opened.mark;
        index = opened.next;

        continue;
      }
    }

    if (source[index] === "`") {
      index = endOfInlineCode(source, index);

      continue;
    }

    if (source.startsWith("||", index) && !escaped(source, index)) {
      const opener = stack.at(-1);

      if (opener !== undefined && !blankBetween(source, opener, index)) {
        stack.pop();

        for (let cursor = opener; cursor < index + 2; cursor += 1) {
          hidden[cursor] = true;
        }
      } else {
        stack.push(index);
      }

      index += 2;

      continue;
    }

    index += 1;
  }

  let out = "";
  let cursor = 0;

  while (cursor < source.length) {
    if (hidden[cursor] === true) {
      out += "spoiler";

      while (cursor < source.length && hidden[cursor] === true) {
        cursor += 1;
      }
    } else {
      out += source[cursor];
      cursor += 1;
    }
  }

  return out;
}

/** The text of sanitized HTML, with each spoiler span replaced by the word "spoiler". */
export function htmlPlainText(html: string): string {
  const document = new DOMParser().parseFromString(html, "text/html");

  for (const node of document.querySelectorAll("[data-spoiler], .spoiler")) {
    if (node.isConnected) {
      node.replaceWith(document.createTextNode("spoiler"));
    }
  }

  return (document.body.textContent ?? "").replace(/\s+\n/g, "\n").trim();
}
