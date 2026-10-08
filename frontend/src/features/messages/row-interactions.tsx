import { useNavigate } from "@tanstack/react-router";
import {
  type AnimationEvent,
  type FocusEvent,
  type KeyboardEvent,
  type MouseEvent,
  type PointerEvent,
  type ReactNode,
  type RefObject,
  Suspense,
  useEffect,
  useRef,
  useState,
} from "react";
import type { EmojiChoice } from "../../lib/emoji/recent.ts";
import { readDurationMs } from "../../motion/durations.ts";
import { prefersReducedMotion } from "../../motion/reduced-motion.ts";
import { lazyForUpdate as lazy } from "../../service-worker/lazy.ts";
import type { MessageDTO } from "../../store/model.ts";
import {
  copyLink,
  copyText,
  markUnread,
  remove,
  togglePin,
  toggleReaction,
  toggleSave,
} from "./commands.ts";
import { startEditing, stopEditing, useEditingId } from "./editing-store.ts";
import { HoverBar } from "./hover-bar.tsx";
import { focusAdjacentRow, focusComposerNear, rowCommand } from "./keyboard.ts";
import { requestListEdge } from "./list-edges.ts";
import type { MenuCommand } from "./message-menu.tsx";
import { trackModality, usingKeyboard } from "./modality.ts";
import type { MessagePermissions } from "./permissions.ts";
import {
  originForKeyboard,
  originFromElement,
  originFromPoint,
  type PopupKind,
  type PopupOrigin,
  type PopupRequest,
  RowPopup,
} from "./row-popup.tsx";
import { useMessagePermissions, useSavedItemId } from "./use-message.ts";

const ForwardDialog = lazy(() => import("./forward-dialog.tsx"));

const DeleteDialog = lazy(() => import("./delete-dialog.tsx"));

/** How long a finger rests on a row before its menu opens (touch has no right click). */
const LONG_PRESS_MS = 500;

/** A press that wanders further than this is a scroll, not a long press. */
const LONG_PRESS_SLOP = 10;

/** The native menu stays for links and media, so "Copy link address" and "Save image" still work. */
const NATIVE_MENU_TARGETS = "a[href], img, video, audio, textarea, input";

/** Longest wait for the button's release before opening anyway. */
const RELEASE_WAIT_MS = 1000;

/**
 * Runs `open` once the pressed pointer is released. A popover opened while the button is still
 * down is light-dismissed by that very release (the press began outside it), so a right click
 * (which fires on press on macOS and Linux) and a long press open on release instead.
 */
function afterRelease(pressed: boolean, open: () => void): void {
  if (!pressed) {
    open();

    return;
  }

  const done = () => {
    window.removeEventListener("pointerup", done, true);
    window.removeEventListener("pointercancel", done, true);
    window.clearTimeout(fallback);
    window.setTimeout(open, 0);
  };

  const fallback = window.setTimeout(done, RELEASE_WAIT_MS);

  window.addEventListener("pointerup", done, true);
  window.addEventListener("pointercancel", done, true);
}

type DialogKind = "forward" | "delete";

interface DialogState {
  readonly kind: DialogKind;
  readonly open: boolean;
  /** Bumps per opening, so each opening starts fresh (a new search, no ticks left over). */
  readonly key: number;
}

export interface RowInteractions {
  readonly permissions: MessagePermissions;
  readonly saved: boolean;
  readonly editing: boolean;
  /** The row's height while it collapses away after a delete, else `null`. */
  readonly removingHeight: number | null;
  readonly rowRef: RefObject<HTMLElement | null>;
  readonly rowProps: {
    readonly onPointerEnter: (event: PointerEvent<HTMLElement>) => void;
    readonly onPointerLeave: () => void;
    readonly onPointerDown: (event: PointerEvent<HTMLElement>) => void;
    readonly onPointerMove: (event: PointerEvent<HTMLElement>) => void;
    readonly onPointerUp: () => void;
    readonly onPointerCancel: () => void;
    readonly onContextMenu: (event: MouseEvent<HTMLElement>) => void;
    readonly onKeyDown: (event: KeyboardEvent<HTMLElement>) => void;
    readonly onFocus: () => void;
    readonly onBlur: (event: FocusEvent<HTMLElement>) => void;
    readonly onAnimationEnd: (event: AnimationEvent<HTMLElement>) => void;
  };
  /** The hover bar, when it should show. */
  readonly bar: ReactNode;
  /** The open popup and dialogs. */
  readonly overlays: ReactNode;
  readonly onAddReaction: (anchor: HTMLElement) => void;
  readonly closeEditor: () => void;
  readonly requestDelete: () => void;
}

/**
 * Everything a message row does besides drawing the message: hover intent for the bar, the
 * right-click and long-press menu, the emoji picker and boost popovers, the row's keys, editing,
 * forwarding and the delete confirmation with its collapse.
 */
export function useRowInteractions(message: MessageDTO, inThread: boolean): RowInteractions {
  const navigate = useNavigate();
  const permissions = useMessagePermissions(message);
  const saved = useSavedItemId(message.id) !== null;
  const editing = useEditingId() === message.id;
  const rowRef = useRef<HTMLElement | null>(null);
  const hoverTimer = useRef(0);
  const pressTimer = useRef(0);
  const pressStart = useRef<{ x: number; y: number } | null>(null);
  const popupCount = useRef(0);
  const returnTo = useRef<HTMLElement | null>(null);
  const [hovered, setHovered] = useState(false);
  const [keyboardFocus, setKeyboardFocus] = useState(false);
  const [popup, setPopup] = useState<PopupRequest | null>(null);
  const [dialog, setDialog] = useState<DialogState | null>(null);
  const [removingHeight, setRemovingHeight] = useState<number | null>(null);

  useEffect(() => {
    trackModality();

    return () => {
      window.clearTimeout(hoverTimer.current);
      window.clearTimeout(pressTimer.current);
    };
  }, []);

  const row = () => rowRef.current;

  /** After a popup or dialog: back to where the person was, or the row, when using the keyboard. */
  const settleFocus = () => {
    const element = row();
    const active = document.activeElement;

    const stranded =
      active === null ||
      active === document.body ||
      !active.isConnected ||
      active.classList.contains("message-popup-anchor");

    if (element === null || !stranded) {
      return;
    }

    if (!usingKeyboard()) {
      if (active instanceof HTMLElement) {
        active.blur();
      }

      return;
    }

    const back = returnTo.current;

    (back?.isConnected === true ? back : element).focus({ preventScroll: true });
  };

  const openPopup = (
    kind: PopupKind,
    origin: PopupOrigin,
    from: HTMLElement | null,
    fromBar = false,
  ) => {
    popupCount.current += 1;
    returnTo.current = from;
    setPopup({ id: popupCount.current, kind, origin, fromBar, keyboard: usingKeyboard() });
  };

  const keyboardOrigin = (): PopupOrigin | null => {
    const element = row();

    return element === null ? null : originForKeyboard(element);
  };

  const openFromKeyboard = (kind: PopupKind) => {
    const origin = keyboardOrigin();

    if (origin !== null) {
      openPopup(kind, origin, row());
    }
  };

  const openDialog = (kind: DialogKind) => {
    setDialog((current) => ({ kind, open: true, key: (current?.key ?? 0) + 1 }));
  };

  const closeDialog = (open: boolean) => {
    if (open) {
      return;
    }

    setDialog((current) => (current === null ? null : { ...current, open: false }));
    window.setTimeout(settleFocus, readDurationMs("--duration-medium-exit") + 20);
  };

  const openThread = () => {
    const threadId = message.thread?.threadId ?? message.threadId;

    if (threadId !== null) {
      void navigate({
        to: "/r/$roomId/t/$threadId",
        params: { roomId: message.roomId, threadId },
      });

      return;
    }

    void navigate({
      to: "/r/$roomId/t/new",
      params: { roomId: message.roomId },
      search: { parent: message.id },
    });
  };

  const react = (choice: EmojiChoice) => {
    if (permissions.react) {
      toggleReaction(message, choice);
    }
  };

  const run = (command: MenuCommand) => {
    switch (command) {
      case "thread":
        if (permissions.thread && !inThread) openThread();

        return;
      case "react":
        if (permissions.react) openFromKeyboardOr("picker");

        return;
      case "boost":
        if (permissions.react) openFromKeyboardOr("boost");

        return;
      case "edit":
        if (permissions.edit) startEditing(message.id, usingKeyboard() ? row() : null);

        return;
      case "copy-text":
        copyText(message);

        return;
      case "copy-link":
        copyLink(message);

        return;
      case "pin":
        if (permissions.pin) togglePin(message);

        return;
      case "save":
        if (permissions.save) toggleSave(message);

        return;
      case "forward":
        if (permissions.forward) openDialog("forward");

        return;
      case "unread":
        if (permissions.markUnread) markUnread(message);

        return;
      case "delete":
        if (permissions.remove) openDialog("delete");

        return;
    }
  };

  /** A follow-up popup (picker or boost from the menu) hangs where the menu did. */
  function openFromKeyboardOr(kind: PopupKind) {
    if (popup !== null) {
      openPopup(kind, popup.origin, returnTo.current);

      return;
    }

    openFromKeyboard(kind);
  }

  const onAddReaction = (anchor: HTMLElement) => {
    const element = row();

    if (element !== null) {
      openPopup("picker", originFromElement(anchor, element, "top-start"), anchor);
    }
  };

  const onPopupClosed = (id: number) => {
    if (popup?.id !== id) {
      return;
    }

    settleFocus();
    setPopup((current) => (current?.id === id ? null : current));
  };

  const confirmDelete = () => {
    const element = row();

    if (element === null || prefersReducedMotion()) {
      void remove(message).catch(() => undefined);

      return;
    }

    setRemovingHeight(element.getBoundingClientRect().height);
  };

  const onAnimationEnd = (event: AnimationEvent<HTMLElement>) => {
    if (event.animationName !== "message-collapse" || event.target !== event.currentTarget) {
      return;
    }

    const element = row();

    if (element?.contains(document.activeElement) === true) {
      if (!focusAdjacentRow(element, "down")) {
        focusAdjacentRow(element, "up");
      }
    }

    remove(message).catch(() => setRemovingHeight(null));
  };

  const closeEditor = () => {
    const target = stopEditing();
    const element = row();

    if (target !== null) {
      target.focus({ preventScroll: true });
    } else if (element !== null) {
      focusComposerNear(element);
    }
  };

  const onKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    if (event.target !== event.currentTarget || editing) {
      return;
    }

    const command = rowCommand(event);
    const element = event.currentTarget;

    if (command === null) {
      return;
    }

    event.preventDefault();

    switch (command) {
      case "up":
        focusAdjacentRow(element, command);

        return;
      case "first":
      case "last":
        if (!requestListEdge(element, command)) focusAdjacentRow(element, command);

        return;
      case "down":
        if (!focusAdjacentRow(element, command)) focusComposerNear(element);

        return;
      case "composer":
        focusComposerNear(element);

        return;
      case "menu":
        openFromKeyboard("menu");

        return;
      case "react":
        if (permissions.react) openFromKeyboard("picker");

        return;
      case "edit":
        if (permissions.edit) startEditing(message.id, element);

        return;
      case "link":
        copyLink(message);

        return;
      case "thread":
      case "pin":
      case "save":
      case "forward":
      case "delete":
        run(command);

        return;
    }
  };

  const onContextMenu = (event: MouseEvent<HTMLElement>) => {
    const target = event.target;
    const element = row();

    if (
      element === null ||
      editing ||
      (target instanceof Element && target.closest(NATIVE_MENU_TARGETS) !== null)
    ) {
      return;
    }

    event.preventDefault();

    // A keyboard's menu key fires contextmenu at (0, 0): hang it from the row instead.
    const origin =
      event.button === 0 && event.clientX === 0 && event.clientY === 0
        ? originForKeyboard(element)
        : originFromPoint(event.clientX, event.clientY, element);

    afterRelease(event.buttons !== 0, () => openPopup("menu", origin, null));
  };

  const cancelPress = () => {
    window.clearTimeout(pressTimer.current);
    pressStart.current = null;
  };

  const onPointerDown = (event: PointerEvent<HTMLElement>) => {
    if (event.pointerType !== "touch" || editing) {
      return;
    }

    const { clientX, clientY } = event;

    pressStart.current = { x: clientX, y: clientY };
    window.clearTimeout(pressTimer.current);

    pressTimer.current = window.setTimeout(() => {
      const element = row();

      pressStart.current = null;

      if (element !== null) {
        const origin = originFromPoint(clientX, clientY, element);

        afterRelease(true, () => openPopup("menu", origin, null));
      }
    }, LONG_PRESS_MS);
  };

  const onPointerMove = (event: PointerEvent<HTMLElement>) => {
    const start = pressStart.current;

    if (
      start !== null &&
      Math.hypot(event.clientX - start.x, event.clientY - start.y) > LONG_PRESS_SLOP
    ) {
      cancelPress();
    }
  };

  const onPointerEnter = (event: PointerEvent<HTMLElement>) => {
    if (event.pointerType !== "mouse") {
      return;
    }

    window.clearTimeout(hoverTimer.current);
    hoverTimer.current = window.setTimeout(
      () => setHovered(true),
      readDurationMs("--duration-intent"),
    );
  };

  const onPointerLeave = () => {
    window.clearTimeout(hoverTimer.current);
    cancelPress();
    setHovered(false);
  };

  const onFocus = () => setKeyboardFocus(usingKeyboard());

  const onBlur = (event: FocusEvent<HTMLElement>) => {
    const next = event.relatedTarget;

    if (!(next instanceof Node && event.currentTarget.contains(next))) {
      setKeyboardFocus(false);
    }
  };

  const showBar =
    !editing && removingHeight === null && (hovered || keyboardFocus || popup !== null);

  const bar = showBar ? (
    <HoverBar
      permissions={permissions}
      saved={saved}
      inThread={inThread}
      menuOpen={popup?.kind === "menu" && popup.fromBar}
      onReact={react}
      onOpenPicker={(button) => {
        const element = row();

        if (element !== null) {
          openPopup("picker", originFromElement(button, element, "bottom-end"), button);
        }
      }}
      onOpenMenu={(button) => {
        const element = row();

        if (element === null) {
          return;
        }

        if (popup?.kind === "menu") {
          setPopup(null);

          return;
        }

        openPopup("menu", originFromElement(button, element, "bottom-end"), button, true);
      }}
      onCommand={run}
    />
  ) : null;

  const overlays = (
    <>
      {popup === null ? null : (
        <RowPopup
          key={popup.id}
          request={popup}
          message={message}
          permissions={permissions}
          saved={saved}
          inThread={inThread}
          onCommand={run}
          onReact={react}
          onClosed={onPopupClosed}
        />
      )}
      {dialog === null ? null : (
        <Suspense fallback={null}>
          {dialog.kind === "forward" ? (
            <ForwardDialog
              key={dialog.key}
              message={message}
              open={dialog.open}
              onOpenChange={closeDialog}
            />
          ) : (
            <DeleteDialog
              key={dialog.key}
              message={message}
              open={dialog.open}
              onOpenChange={closeDialog}
              onConfirm={confirmDelete}
            />
          )}
        </Suspense>
      )}
    </>
  );

  return {
    permissions,
    saved,
    editing,
    removingHeight,
    rowRef,
    rowProps: {
      onPointerEnter,
      onPointerLeave,
      onPointerDown,
      onPointerMove,
      onPointerUp: cancelPress,
      onPointerCancel: cancelPress,
      onContextMenu,
      onKeyDown,
      onFocus,
      onBlur,
      onAnimationEnd,
    },
    bar,
    overlays,
    onAddReaction,
    closeEditor,
    requestDelete: () => openDialog("delete"),
  };
}
