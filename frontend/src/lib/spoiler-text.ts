/**
 * Plain-text previews say "spoiler" instead of the hidden words.
 *
 * The server's renderer decides what a spoiler is (`||…||` in crates/richtext, one level: a
 * spoiler inside another joins it). The client does not parse Markdown for it. A preview comes
 * from the rendered HTML, whose spoiler spans carry `data-spoiler`. Where there is no HTML (a
 * scheduled message), the server sends a redacted `excerpt` instead.
 */

/**
 * A message's plain text: its Markdown when that has no `||` and so no spoiler, else the text of
 * its rendered HTML with each spoiler as the word "spoiler".
 */
export function messagePlainText(markdown: string | null, html: string): string {
  if (markdown !== null && !markdown.includes("||")) {
    return markdown;
  }

  return htmlPlainText(html);
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
