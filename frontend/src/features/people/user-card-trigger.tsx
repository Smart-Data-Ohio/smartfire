import { type MouseEvent, type PointerEvent, type ReactNode, useEffect, useRef } from "react";
import type { Placement } from "../../lib/anchor.ts";
import { AgentProfileLink, useAgentIdOf } from "../agents/agent-link.tsx";
import {
  hoverEnter,
  hoverLeave,
  openByClick,
  releaseAnchor,
  useUserCardExpanded,
} from "./user-card-store.ts";

interface TriggerProps {
  readonly userId: number;
  /** Where the card hangs off the trigger: below a name, beside an avatar. */
  readonly placement?: Placement;
  readonly children: ReactNode;
}

interface ButtonTriggerProps extends TriggerProps {
  readonly className?: string;
  /**
   * A second trigger for the same person beside their name (the avatar): out of the tab order and
   * hidden from assistive tech, so the name is the one stop. A click still opens the card, and
   * never focuses it.
   */
  readonly duplicate?: boolean;
}

/** Closes the card when the element it hangs off goes away. */
function useReleaseOnUnmount(ref: { readonly current: Element | null }): void {
  useEffect(() => {
    const element = ref.current;

    return () => {
      if (element !== null) {
        releaseAnchor(element);
      }
    };
  }, [ref]);
}

/**
 * A name or avatar that opens someone's card: after a moment's hover with a mouse, or at once on a
 * click, a tap, Enter or Space (focus then moves into the card, and Esc brings it back here).
 */
export function UserCardTrigger({
  userId,
  placement = "bottom-start",
  className,
  duplicate = false,
  children,
}: ButtonTriggerProps) {
  const ref = useRef<HTMLButtonElement | null>(null);

  useReleaseOnUnmount(ref);

  const expanded = useUserCardExpanded(ref);
  const target = (anchor: HTMLElement) => ({ userId, anchor, placement });

  return (
    <button
      ref={ref}
      type="button"
      className={className === undefined ? "user-card-trigger" : `user-card-trigger ${className}`}
      aria-haspopup="dialog"
      aria-expanded={duplicate ? undefined : expanded}
      data-user-card-trigger={userId}
      onPointerEnter={(event: PointerEvent<HTMLButtonElement>) =>
        hoverEnter(target(event.currentTarget), event.pointerType)
      }
      onPointerLeave={hoverLeave}
      onMouseDown={duplicate ? (event) => event.preventDefault() : undefined}
      onClick={(event: MouseEvent<HTMLButtonElement>) => openByClick(target(event.currentTarget))}
      tabIndex={duplicate ? -1 : undefined}
      aria-hidden={duplicate || undefined}
    >
      {children}
    </button>
  );
}

/**
 * Hover intent alone, for a trigger that already does something on click (a link to an agent's
 * profile, a row that opens a conversation): it never nests a button in a link or a button. The
 * wrapper takes no box (`display: contents`); the card hangs off its first element.
 */
export function UserCardHover({ userId, placement = "bottom-start", children }: TriggerProps) {
  const ref = useRef<HTMLSpanElement | null>(null);

  useReleaseOnUnmount(ref);

  const onPointerEnter = (event: PointerEvent<HTMLSpanElement>) => {
    const host = event.currentTarget;
    const first = host.firstElementChild;

    hoverEnter(
      { userId, anchor: first instanceof HTMLElement ? first : host, placement },
      event.pointerType,
    );
  };

  return (
    <span
      ref={ref}
      className="user-card-hover"
      onPointerEnter={onPointerEnter}
      onPointerLeave={hoverLeave}
    >
      {children}
    </span>
  );
}

/**
 * An author's name or avatar: an agent's still opens its profile on click (the card shows on
 * hover), anyone else's opens their card.
 */
export function AuthorCardTrigger({
  userId,
  placement = "bottom-start",
  duplicate = false,
  children,
}: Omit<ButtonTriggerProps, "className">) {
  const agentId = useAgentIdOf(userId);

  if (agentId !== null) {
    return (
      <UserCardHover userId={userId} placement={placement}>
        <AgentProfileLink userId={userId} duplicate={duplicate}>
          {children}
        </AgentProfileLink>
      </UserCardHover>
    );
  }

  return (
    <UserCardTrigger userId={userId} placement={placement} duplicate={duplicate}>
      {children}
    </UserCardTrigger>
  );
}
