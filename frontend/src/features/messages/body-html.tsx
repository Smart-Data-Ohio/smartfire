import type { ComponentProps } from "react";
import { inlineMentions } from "../../lib/body-html.ts";
// The message body styles, for pages outside a room (the Slack plan's samples).
import "../room/room.css";

type BodyHtmlProps = {
  readonly html: string;
  readonly className: string;
} & Omit<ComponentProps<"div">, "children" | "className" | "dangerouslySetInnerHTML">;

/**
 * A message body as the server rendered it, mentions made inline. Only for HTML from the server's
 * message presentation (crates/richtext): its sanitizer is what makes the markup safe to insert.
 * Any other attribute (a `data-*` state, say) goes on the body's element as given.
 */
export function BodyHtml({ html, className, ...rest }: BodyHtmlProps) {
  return (
    <div
      className={className}
      {...rest}
      // biome-ignore lint/security/noDangerouslySetInnerHtml: bodyHtml is the server's sanitizer output (crates/richtext), the HTML the classic views render
      dangerouslySetInnerHTML={{ __html: inlineMentions(html) }}
    />
  );
}
