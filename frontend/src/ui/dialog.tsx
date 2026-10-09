import {
  type KeyboardEvent,
  type MouseEvent,
  type ReactNode,
  type SyntheticEvent,
  useId,
  useLayoutEffect,
  useRef,
} from "react";
import { duringAppFocus } from "../lib/reader-focus.ts";
import { usePresence } from "../motion/presence.ts";
import { IconButton } from "./icon-button.tsx";
import "./dialog.css";

interface DialogProps {
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
  readonly title: string;
  readonly description?: ReactNode;
  /** "alertdialog" for confirmations: no close button, and a backdrop click doesn't dismiss. */
  readonly role?: "dialog" | "alertdialog";
  readonly size?: "sm" | "md";
  readonly footer?: ReactNode;
  readonly children?: ReactNode;
  /** Where focus goes on close when whatever opened the dialog has gone (a deleted item's menu). */
  readonly returnFocus?: () => HTMLElement | null;
  /**
   * Close onto `returnFocus` even when the opener is still there: for a dialog whose opener is
   * incidental (it opened with the page, over whatever had autofocus), and whose `returnFocus`
   * knows the right place (and may move focus there itself once it mounts).
   */
  readonly returnFocusFirst?: boolean;
  /** Called once the dialog has finished closing (its exit animation done): drop what it showed. */
  readonly onExited?: () => void;
}

const FOCUSABLE = [
  "a[href]",
  "button:not([disabled])",
  "input:not([disabled])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  '[tabindex]:not([tabindex="-1"])',
].join(", ");

function focusables(root: HTMLElement): HTMLElement[] {
  return [...root.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(
    (element) => element.closest("[hidden], [inert], [popover]") === null,
  );
}

function initialFocus(dialog: HTMLElement): HTMLElement {
  const preferred = dialog.querySelector<HTMLElement>("[data-autofocus]");

  if (preferred !== null) {
    return preferred;
  }

  const content = dialog.querySelector<HTMLElement>(".dialog-body, .dialog-footer");
  const inContent = content === null ? undefined : focusables(content)[0];

  return inContent ?? focusables(dialog)[0] ?? dialog;
}

/**
 * A modal dialog on the native <dialog>: showModal() puts it in the top layer and makes the page
 * behind it inert. On top of that it keeps Tab inside, closes on Esc (and on a backdrop click for
 * plain dialogs), plays the modal recipe's exit before closing, and returns focus to whatever
 * opened it. Where showModal() is missing it falls back to the open attribute plus aria-modal.
 */
export function Dialog({
  open,
  onOpenChange,
  title,
  description,
  role = "dialog",
  size = "md",
  footer,
  children,
  returnFocus,
  returnFocusFirst = false,
  onExited,
}: DialogProps) {
  const id = useId();
  const returnFocusRef = useRef(returnFocus);
  const returnFocusFirstRef = useRef(returnFocusFirst);
  const onExitedRef = useRef(onExited);
  const presence = usePresence<HTMLDialogElement>(open);
  const titleId = `${id}-title`;
  const descriptionId = `${id}-description`;

  useLayoutEffect(() => {
    returnFocusRef.current = returnFocus;
    returnFocusFirstRef.current = returnFocusFirst;
    onExitedRef.current = onExited;
  });

  useLayoutEffect(() => {
    const dialog = presence.ref.current;

    if (!presence.mounted || dialog === null) {
      return;
    }

    const opener = document.activeElement;

    if ("showModal" in dialog && !dialog.open) {
      dialog.showModal();
    } else {
      dialog.setAttribute("open", "");
    }

    initialFocus(dialog).focus();

    return () => {
      if ("close" in dialog && dialog.open) {
        dialog.close();
      }

      // The body isn't an opener (a dialog that opened with the page): use the fallback instead.
      // Restoring focus is the app's, even when Escape or the close click was a moment ago.
      duringAppFocus(() => {
        if (returnFocusFirstRef.current) {
          returnFocusRef.current?.()?.focus({ preventScroll: true });
        } else if (
          opener instanceof HTMLElement &&
          opener !== document.body &&
          opener.isConnected
        ) {
          opener.focus({ preventScroll: true });
        } else {
          returnFocusRef.current?.()?.focus({ preventScroll: true });
        }
      });

      onExitedRef.current?.();
    };
  }, [presence.mounted, presence.ref]);

  const close = () => onOpenChange(false);

  const onKeyDown = (event: KeyboardEvent<HTMLDialogElement>) => {
    if (event.key === "Escape") {
      // Handled here so the exit can play; cancelling also stops the native cancel/close.
      event.preventDefault();
      event.stopPropagation();
      close();

      return;
    }

    if (event.key !== "Tab") {
      return;
    }

    const items = focusables(event.currentTarget);
    const first = items[0];
    const last = items.at(-1);

    if (first === undefined || last === undefined) {
      event.preventDefault();

      return;
    }

    const active = document.activeElement;
    const inside = active instanceof Node && event.currentTarget.contains(active);

    if (event.shiftKey && (active === first || !inside)) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && (active === last || !inside)) {
      event.preventDefault();
      first.focus();
    }
  };

  const onCancel = (event: SyntheticEvent<HTMLDialogElement>) => {
    event.preventDefault();
    close();
  };

  const onClick = (event: MouseEvent<HTMLDialogElement>) => {
    // The surface fills the dialog, so a click on the dialog element itself is on the backdrop.
    if (event.target === event.currentTarget && role === "dialog") {
      close();
    }
  };

  if (!presence.mounted) {
    return null;
  }

  return (
    <dialog
      ref={presence.ref}
      className="dialog t-modal"
      role={role === "alertdialog" ? "alertdialog" : undefined}
      aria-modal="true"
      aria-labelledby={titleId}
      aria-describedby={description === undefined ? undefined : descriptionId}
      data-state={presence.state}
      data-size={size}
      onKeyDown={onKeyDown}
      onCancel={onCancel}
      onClick={onClick}
    >
      <div className="dialog-surface">
        <header className="dialog-header">
          <h2 id={titleId} className="dialog-title">
            {title}
          </h2>
          {role === "dialog" ? (
            <IconButton
              icon="x"
              label="Close"
              size="sm"
              onClick={close}
              tooltipPlacement="bottom"
            />
          ) : null}
        </header>
        {description === undefined ? null : (
          <p id={descriptionId} className="dialog-description">
            {description}
          </p>
        )}
        {children === undefined ? null : <div className="dialog-body">{children}</div>}
        {footer === undefined ? null : <footer className="dialog-footer">{footer}</footer>}
      </div>
    </dialog>
  );
}
