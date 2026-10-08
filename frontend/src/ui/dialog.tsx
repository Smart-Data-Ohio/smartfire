import {
  type KeyboardEvent,
  type MouseEvent,
  type PointerEvent,
  type ReactNode,
  type SyntheticEvent,
  useId,
  useLayoutEffect,
  useRef,
} from "react";
import { COARSE_QUERY, PHONE_QUERY } from "../lib/breakpoints.ts";
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
  /**
   * Whether the form holds unsaved input. A phone sheet's swipe down then springs back instead of
   * closing; Esc and the close button still ask `onOpenChange`. Left out, anything typed or
   * chosen in the dialog since it opened counts.
   */
  readonly dirty?: boolean;
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

/** Whether `query` matches; never, where there's no matchMedia (jsdom leaves it undefined). */
function matches(query: string): boolean {
  return window.matchMedia?.(query).matches === true;
}

/** Whether focusing `element` brings up the on-screen keyboard. */
function takesText(element: HTMLElement): boolean {
  if (element instanceof HTMLInputElement) {
    return !["button", "checkbox", "color", "file", "radio", "range", "reset", "submit"].includes(
      element.type,
    );
  }

  return element instanceof HTMLTextAreaElement || element.isContentEditable;
}

/**
 * Whether `element` may take a dialog's opening focus. Anything may, with a mouse or a keyboard.
 * On a touch screen a focused field raises the keyboard over the dialog before anyone has read
 * it, and nobody needs a ring on whatever happens to come first, so only an element marked
 * `data-autofocus` may: one that raises no keyboard (a confirmation's Cancel), or a field that's
 * the whole point (`data-autofocus="always"`: a search to type in).
 */
function mayTakeOpeningFocus(element: HTMLElement): boolean {
  if (!matches(COARSE_QUERY)) {
    return true;
  }

  const mark = element.dataset.autofocus;

  return mark === "always" || (mark !== undefined && !takesText(element));
}

/** The `data-autofocus` element, else the first control; the dialog itself where that may not. */
function initialFocus(dialog: HTMLElement): HTMLElement {
  const preferred = dialog.querySelector<HTMLElement>("[data-autofocus]");
  const content = dialog.querySelector<HTMLElement>(".dialog-body, .dialog-footer");
  const inContent = content === null ? undefined : focusables(content)[0];
  const target = preferred ?? inContent ?? focusables(dialog)[0] ?? dialog;

  return mayTakeOpeningFocus(target) ? target : dialog;
}

/** Gives `field` the opening focus, by initialFocus's rule: for a form that loads late. */
export function focusOnOpen(field: HTMLElement): void {
  if (mayTakeOpeningFocus(field)) {
    field.focus();
  }
}

/** How far down (px) a released drag closes the sheet. */
const SWIPE_DISTANCE = 96;

/** How fast (px/ms) a shorter drag must be going when let go to count as a flick that closes it. */
const SWIPE_VELOCITY = 0.5;

/** The flick's speed is measured over the drag's last this-many ms, not between two moves. */
const SWIPE_WINDOW = 100;

/** How far (px) the finger moves before the drag is judged a swipe down, or not one. */
const SWIPE_SLOP = 8;

/** How much of the finger's travel a sheet that won't close (unsaved input) follows. */
const SWIPE_RESISTANCE = 0.3;

interface Drag {
  readonly pointerId: number;
  readonly startX: number;
  readonly startY: number;
  /** Unsaved input when the drag began: the sheet follows reluctantly and springs back. */
  readonly held: boolean;
  /** Past the slop, mostly down: the sheet follows. Until then it may still turn out a sideways drag. */
  engaged: boolean;
  /** Where the finger was and when, over the last SWIPE_WINDOW ms (and the start). */
  samples: { readonly y: number; readonly time: number }[];
}

/** Whether the sheet's content is scrolled: a swipe down then belongs to the content, not the sheet. */
function scrolled(sheet: HTMLElement): boolean {
  return [...sheet.querySelectorAll<HTMLElement>(".dialog-body, .dialog-body *")].some(
    (element) => element.scrollTop > 0,
  );
}

/**
 * Swipe down to dismiss a phone sheet, from its grabber and header. It starts only with the
 * content at its top and no text selected, and only as a drag that is mostly downward. The sheet
 * follows the finger (the CSS `translate`, so the modal recipe's transform stays free for the
 * exit); a drag past SWIPE_DISTANCE or a downward flick asks to close it. A sheet with unsaved
 * input (`held`), a short drag, or a close the owner refuses (a form still sending) springs back.
 */
function useSwipeDown(onClose: () => void, held: () => boolean) {
  const drag = useRef<Drag | null>(null);

  const sheetOf = (event: PointerEvent<HTMLElement>) =>
    event.currentTarget.closest<HTMLElement>(".dialog");

  const springBack = (sheet: HTMLElement) => {
    delete sheet.dataset.dragging;
    sheet.style.removeProperty("translate");
  };

  const end = (event: PointerEvent<HTMLElement>, close: boolean) => {
    const sheet = sheetOf(event);

    drag.current = null;

    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }

    if (sheet === null) {
      return;
    }

    if (!close) {
      springBack(sheet);

      return;
    }

    delete sheet.dataset.dragging;
    onClose();
    // The close plays its exit from where the finger let go. The owner may refuse it (a form
    // still sending): then the dialog is still open by the next frame, and the sheet goes back.
    requestAnimationFrame(() => {
      if (sheet.isConnected && sheet.dataset.state !== "closing") {
        springBack(sheet);
      }
    });
  };

  return {
    onPointerDown: (event: PointerEvent<HTMLElement>) => {
      const target = event.target instanceof Element ? event.target : null;
      const sheet = sheetOf(event);

      if (
        event.pointerType === "mouse" ||
        !event.isPrimary ||
        sheet === null ||
        target?.closest("button, a, input, select, textarea") ||
        !matches(PHONE_QUERY) ||
        window.getSelection()?.isCollapsed === false ||
        scrolled(sheet)
      ) {
        return;
      }

      drag.current = {
        pointerId: event.pointerId,
        startX: event.clientX,
        startY: event.clientY,
        held: held(),
        engaged: false,
        samples: [{ y: event.clientY, time: event.timeStamp }],
      };
      event.currentTarget.setPointerCapture(event.pointerId);
    },
    onPointerMove: (event: PointerEvent<HTMLElement>) => {
      const current = drag.current;
      const sheet = sheetOf(event);

      if (current === null || current.pointerId !== event.pointerId || sheet === null) {
        return;
      }

      const dx = event.clientX - current.startX;
      const dy = event.clientY - current.startY;

      if (!current.engaged) {
        if (Math.hypot(dx, dy) < SWIPE_SLOP) {
          return;
        }

        // Sideways or upward: not a swipe to dismiss, whatever the finger does next.
        if (dy <= Math.abs(dx)) {
          end(event, false);

          return;
        }

        current.engaged = true;
      }

      current.samples = [
        ...current.samples.filter((sample) => sample.time >= event.timeStamp - SWIPE_WINDOW),
        { y: event.clientY, time: event.timeStamp },
      ];
      sheet.dataset.dragging = "";
      sheet.style.translate = `0 ${Math.max(0, dy) * (current.held ? SWIPE_RESISTANCE : 1)}px`;
    },
    onPointerUp: (event: PointerEvent<HTMLElement>) => {
      const current = drag.current;

      if (current === null || current.pointerId !== event.pointerId) {
        return;
      }

      const distance = event.clientY - current.startY;
      const from = current.samples[0] ?? { y: current.startY, time: event.timeStamp };
      // At least a frame: two moves a millisecond apart aren't a flick.
      const speed = (event.clientY - from.y) / Math.max(16, event.timeStamp - from.time);
      const far = distance >= SWIPE_DISTANCE || (distance > 0 && speed >= SWIPE_VELOCITY);

      end(event, current.engaged && !current.held && far);
    },
    onPointerCancel: (event: PointerEvent<HTMLElement>) => {
      if (drag.current?.pointerId === event.pointerId) {
        end(event, false);
      }
    },
  };
}

/**
 * A modal dialog on the native <dialog>: showModal() puts it in the top layer and makes the page
 * behind it inert. On top of that it keeps Tab inside, closes on Esc (and on a backdrop click for
 * plain dialogs), plays the modal recipe's exit before closing, and returns focus to whatever
 * opened it. Where showModal() is missing it falls back to the open attribute plus aria-modal.
 *
 * On a phone it is a sheet (dialog.css): full height, or bottom-anchored at `size="sm"`, with its
 * header and footer pinned and the footer kept above the keyboard. Plain dialogs there get a
 * grabber and swipe down to close. An action row inside the body (a form's own buttons) pins the
 * same way with `data-dialog-actions`.
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
  dirty,
}: DialogProps) {
  const id = useId();
  const returnFocusRef = useRef(returnFocus);
  const returnFocusFirstRef = useRef(returnFocusFirst);
  const onExitedRef = useRef(onExited);
  const dirtyRef = useRef(dirty);
  // Typed or chosen since the dialog opened: the unsaved input `dirty` stands for when left out.
  const editedRef = useRef(false);
  const presence = usePresence<HTMLDialogElement>(open);
  const titleId = `${id}-title`;
  const descriptionId = `${id}-description`;

  useLayoutEffect(() => {
    returnFocusRef.current = returnFocus;
    returnFocusFirstRef.current = returnFocusFirst;
    onExitedRef.current = onExited;
    dirtyRef.current = dirty;
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

    editedRef.current = false;
    initialFocus(dialog).focus();

    return () => {
      if ("close" in dialog && dialog.open) {
        dialog.close();
      }

      // The body isn't an opener (a dialog that opened with the page): use the fallback instead.
      if (returnFocusFirstRef.current) {
        returnFocusRef.current?.()?.focus({ preventScroll: true });
      } else if (opener instanceof HTMLElement && opener !== document.body && opener.isConnected) {
        opener.focus({ preventScroll: true });
      } else {
        returnFocusRef.current?.()?.focus({ preventScroll: true });
      }

      onExitedRef.current?.();
    };
  }, [presence.mounted, presence.ref]);

  const close = () => onOpenChange(false);
  const swipe = useSwipeDown(close, () => dirtyRef.current ?? editedRef.current);

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
      tabIndex={-1}
      onKeyDown={onKeyDown}
      onInput={() => {
        editedRef.current = true;
      }}
      onCancel={onCancel}
      onClick={onClick}
    >
      <div className="dialog-surface">
        <header className="dialog-header" {...(role === "dialog" ? swipe : {})}>
          {role === "dialog" ? <span className="dialog-grabber" aria-hidden="true" /> : null}
          <h2 id={titleId} className="dialog-title">
            {title}
          </h2>
          {role === "dialog" ? (
            <IconButton
              icon="x"
              label="Close"
              size="sm"
              className="dialog-close"
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
