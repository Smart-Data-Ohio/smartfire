/**
 * Plain-text previews say "spoiler" instead of the hidden words.
 *
 * The server's renderer decides what a spoiler is (`||…||` in crates/richtext, one level: a
 * spoiler inside another joins it). The client does not parse Markdown for it again. A preview
 * comes from the rendered HTML, whose spoiler spans carry `data-spoiler`. Only where there is no
 * HTML (a scheduled message's draft) is the Markdown redacted, and then broadly: anything that
 * could be a spoiler is hidden, even if the renderer would show it.
 */

/** Two blocks: inline spans, spoilers included, never cross a blank line. */
const BLANK_LINE = /(\n[ \t]*\n)/;

/**
 * A link reference definition (`[label]: url "title"`), with a title on the next line. It shows
 * a URL in a Markdown excerpt, so once a message has a spoiler every definition is hidden.
 */
const DEFINITION = /^ {0,3}\[(?:[^\]\\\n]|\\.)+\]:[^\n]*(?:\n[ \t]*["'(][^\n]*)?/gm;

/** The index just past the destination of the link whose label ends at `at` - 1. */
function linkEnd(block: string, at: number): number {
  if (block[at] === "[") {
    const close = block.indexOf("]", at);

    return close === -1 ? block.length : close + 1;
  }

  if (block[at] !== "(") {
    return at;
  }

  let depth = 0;
  let quote: string | null = null;

  for (let index = at; index < block.length; index += 1) {
    const char = block[index];

    if (quote !== null) {
      quote = char === quote ? null : quote;
    } else if (char === '"' || char === "'") {
      quote = char;
    } else if (char === "(") {
      depth += 1;
    } else if (char === ")") {
      depth -= 1;

      if (depth === 0) {
        return index + 1;
      }
    }
  }

  return block.length;
}

/**
 * One block's preview: from the first `||` to the last becomes "spoiler", widened to the whole of
 * any link or image whose label holds that span (its `[`, and its `(url "title")` or `[ref]`).
 */
function redactBlock(block: string): string {
  const first = block.indexOf("||");
  const last = block.lastIndexOf("||");

  if (first === -1 || last < first + 2) {
    return block;
  }

  let start = first;
  let depth = 0;

  // Back to the outermost `[` (or `![`) still open where the spoiler starts.
  for (let index = first - 1; index >= 0; index -= 1) {
    if (block[index] === "]") {
      depth += 1;
    } else if (block[index] === "[") {
      if (depth === 0) {
        start = block[index - 1] === "!" ? index - 1 : index;
      } else {
        depth -= 1;
      }
    }
  }

  let end = last + 2;

  depth = 0;

  // On past each `]` that closes such a label, and the destination or reference after it.
  for (let index = end; index < block.length; index += 1) {
    if (block[index] === "[") {
      depth += 1;
    } else if (block[index] === "]") {
      if (depth > 0) {
        depth -= 1;
      } else {
        end = linkEnd(block, index + 1);
        index = end - 1;
      }
    }
  }

  return `${block.slice(0, start)}spoiler${block.slice(end)}`;
}

/**
 * Markdown with no rendered HTML, as a preview. In each block, everything from the first `||` to
 * the last becomes the word "spoiler". That covers every spoiler the renderer could make there,
 * whatever escapes, code spans or nesting it holds. A link or image whose label holds one is
 * hidden whole, URL and title too, and so is every reference definition. A block with one `||`
 * (no pair) is unchanged. crates/richtext's `markdown::redact_spoilers` does the same.
 */
export function redactMarkdownSpoilers(source: string): string {
  const blocks = source.split(BLANK_LINE);

  if (!blocks.some((block) => redactBlock(block) !== block)) {
    return source;
  }

  return source.replace(DEFINITION, "spoiler").split(BLANK_LINE).map(redactBlock).join("");
}

/**
 * A message's plain text: its Markdown when that has no `||` and so no spoiler, else the text of
 * its rendered HTML with each spoiler as the word "spoiler". With no HTML either, the Markdown is
 * redacted broadly (`redactMarkdownSpoilers`).
 */
export function messagePlainText(markdown: string | null, html: string): string {
  if (markdown !== null && !markdown.includes("||")) {
    return markdown;
  }

  if (markdown === null || html.trim() !== "") {
    return htmlPlainText(html);
  }

  return redactMarkdownSpoilers(markdown);
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
