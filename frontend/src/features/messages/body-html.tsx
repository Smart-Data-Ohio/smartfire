import { inlineMentions } from "../../lib/body-html.ts";
// The message body styles, for pages outside a room (the Slack plan's samples).
import "../room/room.css";

/**
 * A message body as the server rendered it, mentions made inline. Only for HTML from the server's
 * message presentation (crates/richtext): its sanitizer is what makes the markup safe to insert.
 */
export function BodyHtml({
  html,
  className,
}: {
  readonly html: string;
  readonly className: string;
}) {
  return (
    <div
      className={className}
      // biome-ignore lint/security/noDangerouslySetInnerHtml: bodyHtml is the server's sanitizer output (crates/richtext), the HTML the classic views render
      dangerouslySetInnerHTML={{ __html: inlineMentions(html) }}
    />
  );
}
