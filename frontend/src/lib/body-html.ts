/**
 * The server renders a mention as `<div class="mention">` inside the paragraph (crates/richtext,
 * Rails' users/_mention). Parsed as HTML, a `<div>` start tag closes the open `<p>`, so the
 * mention would drop onto a line of its own and split the sentence. Renaming the wrapper to a
 * `<span>` keeps it inline; the attributes and contents (an avatar link and a name button, no
 * nested div) stay as they are.
 */
const MENTION_BLOCK = /<div class="mention([^"]*)"([^>]*)>([\s\S]*?)<\/div>/g;

/** `bodyHtml` with every mention wrapper inline, ready for `innerHTML`. */
export function inlineMentions(html: string): string {
  return html.includes('<div class="mention')
    ? html.replace(MENTION_BLOCK, '<span class="mention$1"$2>$3</span>')
    : html;
}
