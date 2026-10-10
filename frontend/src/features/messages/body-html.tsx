import { type ComponentProps, useLayoutEffect, useRef } from "react";
import { inlineMentions } from "../../lib/body-html.ts";
import { enhanceCodeBlocks } from "../../lib/code-highlight/code-blocks.ts";
// The message body styles, for pages outside a room (the Slack plan's samples).
import "../room/room.css";
import { bindSpoilers } from "./spoilers.ts";

type BodyHtmlProps = {
  readonly html: string;
  readonly className: string;
} & Omit<ComponentProps<"div">, "children" | "className" | "dangerouslySetInnerHTML" | "ref">;

/**
 * A message body as the server rendered it, mentions made inline. Only for HTML from the server's
 * message presentation (crates/richtext): its sanitizer is what makes the markup safe to insert.
 * Any other attribute (a `data-*` state, say) goes on the body's element as given.
 *
 * Code blocks get a Copy button and syntax colours (lib/code-highlight) after mount, once per
 * `html`: React rewrites the markup only when `html` changes, and that is when they are redone.
 * A cached colouring (a row the timeline mounts again) applies before paint.
 *
 * A `||spoiler||` starts covered. Clicking it, or Enter or Space while it is focused, reveals
 * that one spoiler and does not bubble into the message row's actions. The composer preview uses
 * this same body, so spoilers there behave the same way.
 */
export function BodyHtml({ html, className, ...rest }: BodyHtmlProps) {
  const ref = useRef<HTMLDivElement>(null);

  useLayoutEffect(() => {
    const root = ref.current;

    if (root === null) {
      return;
    }

    const unbind = bindSpoilers(root);
    const removeCode = html.includes("<pre") ? enhanceCodeBlocks(root) : undefined;

    return () => {
      unbind();
      removeCode?.();
    };
  }, [html]);

  return (
    <div
      ref={ref}
      className={className}
      {...rest}
      // biome-ignore lint/security/noDangerouslySetInnerHtml: bodyHtml is the server's sanitizer output (crates/richtext), the HTML the classic views render
      dangerouslySetInnerHTML={{ __html: inlineMentions(html) }}
    />
  );
}
