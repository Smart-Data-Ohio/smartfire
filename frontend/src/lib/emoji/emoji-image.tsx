import type { ComponentProps } from "react";
import { useReducedMotion } from "../../motion/reduced-motion.ts";

/** A workspace icon's own URL. Brand icons and other images never move. */
const WORKSPACE_ICON = /^\/icons\/[a-z0-9_]+$/;

/** A workspace icon in message HTML, as crates/richtext renders `:name:`. */
const BODY_ICON = /<img\b[^>]*\bclass="icon icon--custom"[^>]*>/g;

const BODY_ICON_SRC = /\bsrc="(\/icons\/[a-z0-9_]+)"/;

/**
 * A custom emoji's first frame. A workspace icon's is always its URL with `?still=1`, whatever
 * still was named: the server answers with an animated icon's first frame and a static icon's
 * original, and a named still may be stale (a catalog cached before a static icon was replaced
 * by an animated one of its name). Any other image uses the named still, else itself.
 */
export function stillSrc(src: string, still?: string | null): string {
  if (WORKSPACE_ICON.test(src)) {
    return `${src}?still=1`;
  }

  return still ?? src;
}

/** Message HTML with every workspace icon on its first frame, for reduced motion. */
export function stillBodyIcons(html: string): string {
  return html.includes("icon--custom")
    ? html.replace(BODY_ICON, (tag) =>
        tag.replace(BODY_ICON_SRC, (_match, src: string) => `src="${stillSrc(src)}"`),
      )
    : html;
}

/**
 * Message HTML as it should show now: under reduced motion, its workspace icons on their first
 * frame. Every place that inserts message HTML goes through this, `BodyHtml` or not.
 */
export function useMotionSafeHtml(html: string): string {
  return useReducedMotion() ? stillBodyIcons(html) : html;
}

type EmojiImageProps = {
  readonly src: string;
  /** The first frame (`stillUrl`), for images other than workspace icons (see `stillSrc`). */
  readonly still?: string | null | undefined;
  /** Shows the first frame while at rest, as a dense grid's cells do until active. */
  readonly resting?: boolean;
} & Omit<ComponentProps<"img">, "src">;

/**
 * A custom emoji's image. An animated one plays; under reduced motion (the OS setting or the
 * user's Smartfire override) it shows its first frame instead.
 */
export function EmojiImage({ src, still, resting = false, alt = "", ...rest }: EmojiImageProps) {
  const reduced = useReducedMotion();

  return <img {...rest} src={reduced || resting ? stillSrc(src, still) : src} alt={alt} />;
}
