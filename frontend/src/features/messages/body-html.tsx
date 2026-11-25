import { type ComponentProps, useCallback, useLayoutEffect, useRef } from "react";
import { enhanceCodeBlocks } from "../../lib/code-highlight/code-blocks.ts";
import { useMotionSafeHtml } from "../../lib/emoji/emoji-image.tsx";
// The message body styles, for pages outside a room (the Slack plan's samples).
import "../room/room.css";
import { useMessageBodyRef } from "../people/mention-cards.ts";

type BodyHtmlProps = {
  readonly html: string;
  readonly className: string;
} & Omit<ComponentProps<"div">, "children" | "className" | "dangerouslySetInnerHTML" | "ref">;

/**
 * A message body as the server rendered it. Only for HTML from the server's
 * message presentation (crates/richtext): its sanitizer is what makes the markup safe to insert.
 * Any other attribute (a `data-*` state, say) goes on the body's element as given.
 *
 * Code blocks get a Copy button and syntax colours (lib/code-highlight) after mount, once per
 * `html`: React rewrites the markup only when `html` changes, and that is when they are redone.
 * A cached colouring (a row the timeline mounts again) applies before paint.
 *
 * A `||spoiler||` starts covered. Clicking it, or Enter or Space while it is focused, reveals
 * that one spoiler and does not bubble into the message row's actions. The composer preview uses
 * this same body, so spoilers there behave the same way. Spoilers and mention chips bind through
 * the same `useMessageBodyRef` ref as every other place that inserts message HTML.
 *
 * A person's mention chip opens their user card on hover or a click; one inside a covered spoiler
 * waits until the spoiler is revealed.
 *
 * Under reduced motion an animated workspace icon shows its first frame.
 */
export function BodyHtml({ html, className, ...rest }: BodyHtmlProps) {
  const ref = useRef<HTMLDivElement>(null);
  // Switching motion inserts the markup again, so spoilers and code blocks follow `shown`.
  const shown = useMotionSafeHtml(html);
  const body = useMessageBodyRef(shown);

  const attach = useCallback(
    (node: HTMLDivElement | null) => {
      ref.current = node;

      return body(node);
    },
    [body],
  );

  useLayoutEffect(() => {
    const root = ref.current;

    if (root === null || !shown.includes("<pre")) {
      return;
    }

    return enhanceCodeBlocks(root);
  }, [shown]);

  return (
    <div
      ref={attach}
      className={className}
      {...rest}
      // biome-ignore lint/security/noDangerouslySetInnerHtml: bodyHtml is the server's sanitizer output (crates/richtext), the HTML the classic views render
      dangerouslySetInnerHTML={{ __html: shown }}
    />
  );
}
