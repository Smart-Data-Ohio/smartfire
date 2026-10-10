import { type ComponentProps, useLayoutEffect, useRef } from "react";
import { inlineMentions } from "../../lib/body-html.ts";
import { enhanceCodeBlocks } from "../../lib/code-highlight/code-blocks.ts";
// The message body styles, for pages outside a room (the Slack plan's samples).
import "../room/room.css";

type BodyHtmlProps = {
  readonly html: string;
  readonly className: string;
} & Omit<ComponentProps<"div">, "children" | "className" | "dangerouslySetInnerHTML" | "ref">;

/** What a screen reader hears while the spoiler is still covered. The words stay in the DOM. */
const SPOILER_LABEL = "Spoiler, activate to reveal";

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

    prepareSpoilers(root);

    const onClick = (event: MouseEvent) => {
      const spoiler = spoilerElement(event.target);

      if (spoiler === null) {
        return;
      }

      // Enter/Space reveals on keydown, then the button's click follows. That click must not
      // reach the message row either. A later mouse click on an open spoiler still can.
      if (spoiler.hasAttribute("data-revealed")) {
        if (event.detail === 0) {
          event.preventDefault();
          event.stopPropagation();
        }

        return;
      }

      reveal(spoiler);
      event.preventDefault();
      event.stopPropagation();
    };

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Enter" && event.key !== " ") {
        return;
      }

      const spoiler = hiddenSpoiler(event.target);

      if (spoiler === null) {
        return;
      }

      reveal(spoiler);
      event.preventDefault();
      event.stopPropagation();
    };

    const onPointerDown = (event: PointerEvent) => {
      if (hiddenSpoiler(event.target) !== null) {
        event.stopPropagation();
      }
    };

    root.addEventListener("click", onClick);
    root.addEventListener("keydown", onKeyDown);
    root.addEventListener("pointerdown", onPointerDown);

    const removeCode = html.includes("<pre") ? enhanceCodeBlocks(root) : undefined;

    return () => {
      root.removeEventListener("click", onClick);
      root.removeEventListener("keydown", onKeyDown);
      root.removeEventListener("pointerdown", onPointerDown);
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

/** Makes each still-hidden spoiler a button the keyboard and screen readers can use. */
function prepareSpoilers(root: HTMLElement): void {
  for (const node of root.querySelectorAll("[data-spoiler]")) {
    if (!(node instanceof HTMLElement) || node.hasAttribute("data-revealed")) {
      continue;
    }

    node.tabIndex = 0;
    node.setAttribute("role", "button");
    node.setAttribute("aria-label", SPOILER_LABEL);
  }
}

/** The spoiler element `target` is in, covered or already revealed. */
function spoilerElement(target: EventTarget | null): HTMLElement | null {
  if (!(target instanceof Element)) {
    return null;
  }

  const spoiler = target.closest("[data-spoiler]");

  return spoiler instanceof HTMLElement ? spoiler : null;
}

/** A spoiler that is still covered. */
function hiddenSpoiler(target: EventTarget | null): HTMLElement | null {
  const spoiler = spoilerElement(target);

  if (spoiler === null || spoiler.hasAttribute("data-revealed")) {
    return null;
  }

  return spoiler;
}

/** Shows this spoiler's text and leaves it as ordinary text. */
function reveal(spoiler: HTMLElement): void {
  spoiler.setAttribute("data-revealed", "");
  spoiler.removeAttribute("aria-label");
  spoiler.removeAttribute("role");
  spoiler.removeAttribute("tabindex");
}
