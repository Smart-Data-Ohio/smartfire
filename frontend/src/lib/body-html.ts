/**
 * The server renders a mention as `<div class="mention">` inside the paragraph (crates/richtext,
 * Rails' users/_mention). Parsed as HTML, a `<div>` start tag closes the open `<p>`, so the
 * mention would drop onto a line of its own and split the sentence. Renaming the wrapper to a
 * `<span>` keeps it inline.
 *
 * Its contents are the classic view's profile-card controls: an avatar link to `/users/:id` (off
 * the SPA, and with no accessible name) and a name button wired to a Stimulus controller that
 * isn't here. Both go inert: the link is dropped and the button becomes a plain `<span>` that
 * keeps its class (the chip's look) and the name.
 */
const MENTION_BLOCK = /<(div|span) class="mention([^"]*)"([^>]*)>([\s\S]*?)<\/\1>/g;

const AVATAR_LINK = /<a\b[^>]*class="[^"]*\bavatar\b[^"]*"[^>]*>[\s\S]*?<\/a>\s*/g;

const NAME_BUTTON = /<button\b[^>]*class="profile-card-name"[^>]*>([\s\S]*?)<\/button>/g;

function inertMention(contents: string): string {
  return contents
    .replace(AVATAR_LINK, "")
    .replace(NAME_BUTTON, '<span class="profile-card-name">$1</span>');
}

/** `bodyHtml` with every mention inline and inert, ready for `innerHTML`. */
export function inlineMentions(html: string): string {
  return html.includes('<div class="mention') || html.includes('<span class="mention')
    ? html.replace(
        MENTION_BLOCK,
        (_match, _tag: string, classes: string, attributes: string, contents: string) =>
          `<span class="mention${classes}"${attributes}>${inertMention(contents)}</span>`,
      )
    : html;
}
