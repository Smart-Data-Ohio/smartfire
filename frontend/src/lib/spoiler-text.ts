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
 * Markdown with no rendered HTML, as a preview. In each block, everything from the first `||` to
 * the last becomes the word "spoiler". That covers every spoiler the renderer could make there,
 * whatever escapes, code spans or nesting it holds. A block with one `||` (no pair) is unchanged.
 */
export function redactMarkdownSpoilers(source: string): string {
  return source
    .split(BLANK_LINE)
    .map((block) => {
      const first = block.indexOf("||");
      const last = block.lastIndexOf("||");

      if (first === -1 || last < first + 2) {
        return block;
      }

      return `${block.slice(0, first)}spoiler${block.slice(last + 2)}`;
    })
    .join("");
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
