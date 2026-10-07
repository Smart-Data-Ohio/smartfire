import {
  createContext,
  type KeyboardEvent,
  type MouseEvent,
  type ReactElement,
  type ReactNode,
  type PointerEvent as ReactPointerEvent,
  type RefObject,
  use,
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import type { Placement } from "../lib/anchor.ts";
import { showPopover, supportsPopover } from "../lib/popover.ts";
import { readDurationMs } from "../motion/durations.ts";
import { usePresence } from "../motion/presence.ts";
import { originFor, useFloating } from "./floating.ts";
import { Icon, type IconName } from "./icons/icon.tsx";
import "./floating.css";
import "./menu.css";

/** Props a menu's trigger must spread onto its button. */
export interface MenuTriggerProps {
  readonly ref: RefObject<HTMLButtonElement | null>;
  readonly id: string;
  readonly "aria-haspopup": "menu";
  readonly "aria-expanded": boolean;
  readonly "aria-controls": string | undefined;
  readonly onClick: (event: MouseEvent<HTMLButtonElement>) => void;
  readonly onKeyDown: (event: KeyboardEvent<HTMLButtonElement>) => void;
}

interface MenuBaseProps {
  readonly trigger: (props: MenuTriggerProps) => ReactElement;
  readonly placement?: Placement;
  /** An accessible name for the menu when the trigger's own name doesn't fit. */
  readonly label?: string;
  readonly children: ReactNode;
}

type MenuProps = MenuBaseProps &
  (
    | { readonly open?: never; readonly onOpenChange?: never }
    | { readonly open: boolean; readonly onOpenChange: (open: boolean) => void }
  );

/** Where focus lands when a menu opens: an item, or the menu itself (pointer opens). */
type FocusTarget = "first" | "last" | "menu" | "none";

type CloseReason = "escape" | "tab" | "select" | "dismiss" | "left";

interface MenuContextValue {
  /** Closes the whole menu tree; with `restoreFocus`, focus goes back to the trigger. */
  readonly closeAll: (restoreFocus: boolean) => void;
}

const MenuContext = createContext<MenuContextValue>({ closeAll: () => {} });

const ITEMS = ':scope > [role^="menuitem"], :scope > [role="group"] > [role^="menuitem"]';

function menuItems(surface: HTMLElement): HTMLElement[] {
  return [...surface.querySelectorAll<HTMLElement>(ITEMS)];
}

function focusItem(item: HTMLElement | undefined): void {
  item?.focus({ preventScroll: true });
}

function itemLabel(item: HTMLElement): string {
  return (item.dataset.label ?? item.textContent ?? "").trim().toLowerCase();
}

/**
 * A dropdown menu (WAI-ARIA menu pattern): arrow keys, Home/End, typeahead, Enter/Space to
 * choose, Esc to close (back to the trigger), Tab to leave, and submenus on ArrowRight or hover.
 * It sits in the top layer as an auto popover, so an outside click dismisses it natively.
 */
export function Menu({
  trigger,
  placement = "bottom-start",
  label,
  children,
  open: controlledOpen,
  onOpenChange,
}: MenuProps) {
  const id = useId();
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const [localOpen, setLocalOpen] = useState(false);
  const open = controlledOpen ?? localOpen;
  const [focusOnOpen, setFocusOnOpen] = useState<FocusTarget>("first");
  const dismissedAt = useRef(Number.NEGATIVE_INFINITY);
  const presence = usePresence<HTMLDivElement>(open);

  const setOpen = (next: boolean) => {
    if (onOpenChange === undefined) {
      setLocalOpen(next);
    } else {
      onOpenChange(next);
    }
  };

  const openWith = (target: FocusTarget) => {
    setFocusOnOpen(target);
    setOpen(true);
  };

  const closeAll = (restoreFocus: boolean) => {
    setOpen(false);

    if (restoreFocus) {
      triggerRef.current?.focus({ preventScroll: true });
    }
  };

  const onClose = (reason: CloseReason) => {
    if (reason === "dismiss") {
      dismissedAt.current = performance.now();
    }

    closeAll(reason === "escape" || reason === "select");
  };

  const triggerProps: MenuTriggerProps = {
    ref: triggerRef,
    id: `${id}-trigger`,
    "aria-haspopup": "menu",
    "aria-expanded": open,
    "aria-controls": presence.mounted ? id : undefined,
    onClick: (event) => {
      if (open) {
        closeAll(false);

        return;
      }

      // The pointerdown that light-dismissed the menu shouldn't reopen it on the same click.
      if (performance.now() - dismissedAt.current < 300) {
        return;
      }

      // A keyboard "click" (Enter/Space) lands on the first item; a pointer click on the menu.
      openWith(event.detail === 0 ? "first" : "menu");
    },
    onKeyDown: (event) => {
      if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault();
        openWith(event.key === "ArrowDown" ? "first" : "last");
      }
    },
  };

  return (
    <>
      {trigger(triggerProps)}
      {presence.mounted ? (
        <MenuContext value={{ closeAll }}>
          <MenuSurface
            id={id}
            surfaceRef={presence.ref}
            anchorRef={triggerRef}
            state={presence.state}
            placement={placement}
            label={label}
            labelledBy={label === undefined ? `${id}-trigger` : undefined}
            focusOnOpen={focusOnOpen}
            onClose={onClose}
          >
            {children}
          </MenuSurface>
        </MenuContext>
      ) : null}
    </>
  );
}

interface MenuSurfaceProps {
  readonly id: string;
  readonly surfaceRef: RefObject<HTMLDivElement | null>;
  readonly anchorRef: RefObject<HTMLElement | null>;
  readonly state: "open" | "closing";
  readonly placement: Placement;
  readonly label: string | undefined;
  readonly labelledBy: string | undefined;
  readonly focusOnOpen: FocusTarget;
  readonly onClose: (reason: CloseReason) => void;
  readonly submenu?: boolean;
  readonly children: ReactNode;
}

function MenuSurface({
  id,
  surfaceRef,
  anchorRef,
  state,
  placement,
  label,
  labelledBy,
  focusOnOpen,
  onClose,
  submenu = false,
  children,
}: MenuSurfaceProps) {
  const typeahead = useRef({ buffer: "", timer: 0 });
  const onCloseRef = useRef(onClose);

  useLayoutEffect(() => {
    onCloseRef.current = onClose;
  });

  useFloating(id, anchorRef, surfaceRef, placement, true);

  // Show in the top layer and move focus in, once, when the surface mounts: where focus lands
  // depends on how the menu was opened, not on later renders.
  // biome-ignore lint/correctness/useExhaustiveDependencies: runs at mount only, by design
  useLayoutEffect(() => {
    const surface = surfaceRef.current;

    if (surface === null) {
      return;
    }

    showPopover(surface);

    const items = menuItems(surface);

    if (focusOnOpen === "first") {
      focusItem(items[0]);
    } else if (focusOnOpen === "last") {
      focusItem(items.at(-1));
    } else if (focusOnOpen === "menu") {
      surface.focus({ preventScroll: true });
    }
  }, [surfaceRef]);

  // Light dismiss: the browser closes auto popovers on an outside click; without the Popover
  // API (old browsers, unit tests) an outside pointerdown does the same.
  useEffect(() => {
    const surface = surfaceRef.current;

    if (surface === null) {
      return;
    }

    const onToggle = (event: Event) => {
      if ("newState" in event && event.newState === "closed") {
        onCloseRef.current("dismiss");
      }
    };

    const onPointerDown = (event: PointerEvent) => {
      const target = event.target;

      if (
        target instanceof Node &&
        !surface.contains(target) &&
        !(anchorRef.current?.contains(target) ?? false)
      ) {
        onCloseRef.current("dismiss");
      }
    };

    if (supportsPopover(surface)) {
      surface.addEventListener("toggle", onToggle);

      return () => surface.removeEventListener("toggle", onToggle);
    }

    if (submenu) {
      return;
    }

    document.addEventListener("pointerdown", onPointerDown);

    return () => document.removeEventListener("pointerdown", onPointerDown);
  }, [surfaceRef, anchorRef, submenu]);

  useEffect(() => () => window.clearTimeout(typeahead.current.timer), []);

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const surface = event.currentTarget;

    // Keys inside a nested submenu belong to that submenu.
    if (!(event.target instanceof Element) || event.target.closest('[role="menu"]') !== surface) {
      return;
    }

    const items = menuItems(surface);
    const active = document.activeElement;
    const index = active instanceof HTMLElement ? items.indexOf(active) : -1;

    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        focusItem(items[(index + 1) % items.length]);
        break;
      case "ArrowUp":
        event.preventDefault();
        focusItem(items[index <= 0 ? items.length - 1 : index - 1]);
        break;
      case "Home":
        event.preventDefault();
        focusItem(items[0]);
        break;
      case "End":
        event.preventDefault();
        focusItem(items.at(-1));
        break;
      case "Escape":
        // Cancelling the keydown also stops the browser's own close request for the popover.
        event.preventDefault();
        event.stopPropagation();
        onClose(submenu ? "left" : "escape");
        break;
      case "ArrowLeft":
        if (submenu) {
          event.preventDefault();
          event.stopPropagation();
          onClose("left");
        }

        break;
      case "Tab":
        onClose("tab");
        break;
      case "Enter":
      case " ":
        if (index >= 0) {
          event.preventDefault();
          items[index]?.click();
        }

        break;
      default:
        if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
          runTypeahead(event.key, items, index);
        }
    }
  };

  const runTypeahead = (key: string, items: HTMLElement[], index: number) => {
    const state = typeahead.current;

    window.clearTimeout(state.timer);
    state.buffer += key.toLowerCase();
    state.timer = window.setTimeout(() => {
      state.buffer = "";
    }, 500);

    // A repeated single letter cycles through the items that start with it.
    const repeated = [...state.buffer].every((character) => character === state.buffer[0]);
    const query = repeated ? state.buffer.slice(0, 1) : state.buffer;
    const start = repeated || index < 0 ? index + 1 : index;
    const ordered = [...items.slice(start), ...items.slice(0, start)];

    focusItem(ordered.find((item) => itemLabel(item).startsWith(query)));
  };

  return (
    <div
      ref={surfaceRef}
      id={id}
      role="menu"
      aria-label={label}
      aria-labelledby={labelledBy}
      aria-orientation="vertical"
      tabIndex={-1}
      popover="auto"
      className="menu floating t-dropdown"
      data-state={state}
      data-placement={placement}
      data-origin={originFor(placement)}
      onKeyDown={onKeyDown}
    >
      {children}
    </div>
  );
}

interface MenuItemProps {
  readonly icon?: IconName;
  /** Shown faint at the end, e.g. ["⌘", "E"]. */
  readonly shortcut?: readonly string[];
  readonly tone?: "danger" | undefined;
  readonly disabled?: boolean;
  readonly onSelect?: () => void;
  readonly children: ReactNode;
}

/** Hovering an item focuses it, so pointer and keyboard share one highlight. */
function focusOnPointer(event: ReactPointerEvent<HTMLElement>): void {
  if (document.activeElement !== event.currentTarget) {
    event.currentTarget.focus({ preventScroll: true });
  }
}

function Shortcut({ keys }: { readonly keys: readonly string[] | undefined }) {
  return keys === undefined ? null : (
    <span className="menu-shortcut" aria-hidden="true">
      {keys.join("")}
    </span>
  );
}

export function MenuItem({
  icon,
  shortcut,
  tone,
  disabled = false,
  onSelect,
  children,
}: MenuItemProps) {
  const { closeAll } = use(MenuContext);

  return (
    // biome-ignore lint/a11y/useKeyWithClickEvents: the menu's keydown handler turns Enter and Space into this click
    <div
      role="menuitem"
      tabIndex={-1}
      className="menu-item"
      data-tone={tone}
      aria-disabled={disabled || undefined}
      onPointerMove={focusOnPointer}
      onClick={() => {
        if (disabled) {
          return;
        }

        onSelect?.();
        closeAll(true);
      }}
    >
      <span className="menu-item-icon">{icon === undefined ? null : <Icon name={icon} />}</span>
      <span className="menu-item-label">{children}</span>
      <Shortcut keys={shortcut} />
    </div>
  );
}

interface MenuCheckboxItemProps {
  readonly checked: boolean;
  readonly onCheckedChange: (checked: boolean) => void;
  readonly children: ReactNode;
}

/** A toggle inside a menu. Choosing it flips the check and leaves the menu open. */
export function MenuCheckboxItem({ checked, onCheckedChange, children }: MenuCheckboxItemProps) {
  return (
    // biome-ignore lint/a11y/useKeyWithClickEvents: the menu's keydown handler turns Enter and Space into this click
    <div
      role="menuitemcheckbox"
      aria-checked={checked}
      tabIndex={-1}
      className="menu-item"
      onPointerMove={focusOnPointer}
      onClick={() => onCheckedChange(!checked)}
    >
      <span className="menu-item-icon">
        <span className="t-icon-swap" data-state={checked ? "a" : "b"}>
          <span className="t-icon" data-icon="a">
            <Icon name="check" />
          </span>
          <span className="t-icon" data-icon="b" />
        </span>
      </span>
      <span className="menu-item-label">{children}</span>
    </div>
  );
}

interface MenuRadioItemProps {
  readonly checked: boolean;
  readonly icon?: IconName;
  /** A faint second line under the label. */
  readonly description?: string;
  readonly onSelect: () => void;
  readonly children: ReactNode;
}

/**
 * One choice of a set inside a menu (wrap the set in a MenuGroup). Choosing it closes the menu,
 * like an item; the chosen one carries a check at the end.
 */
export function MenuRadioItem({
  checked,
  icon,
  description,
  onSelect,
  children,
}: MenuRadioItemProps) {
  const { closeAll } = use(MenuContext);

  return (
    // biome-ignore lint/a11y/useKeyWithClickEvents: the menu's keydown handler turns Enter and Space into this click
    <div
      role="menuitemradio"
      aria-checked={checked}
      tabIndex={-1}
      className="menu-item"
      onPointerMove={focusOnPointer}
      onClick={() => {
        onSelect();
        closeAll(true);
      }}
    >
      <span className="menu-item-icon menu-item-lead">
        {icon === undefined ? null : <Icon name={icon} />}
      </span>
      <span className="menu-item-label">
        {children}
        {description === undefined ? null : (
          <span className="menu-item-description">{description}</span>
        )}
      </span>
      <span className="menu-item-check" aria-hidden="true">
        {checked ? <Icon name="check" /> : null}
      </span>
    </div>
  );
}

export function MenuSeparator() {
  return <hr className="menu-separator" />;
}

export function MenuGroup({
  label,
  children,
}: {
  readonly label: string;
  readonly children: ReactNode;
}) {
  const id = useId();

  return (
    // biome-ignore lint/a11y/useSemanticElements: a <fieldset> isn't allowed inside role="menu"; ARIA groups are
    <div role="group" aria-labelledby={id} className="menu-group">
      <div id={id} className="menu-group-label" aria-hidden="true">
        {label}
      </div>
      {children}
    </div>
  );
}

interface SubMenuProps {
  readonly label: string;
  readonly icon?: IconName;
  readonly children: ReactNode;
}

/** An item that opens a nested menu: ArrowRight, Enter or Space, or a short hover. */
export function SubMenu({ label, icon, children }: SubMenuProps) {
  const id = useId();
  const { closeAll } = use(MenuContext);
  const itemRef = useRef<HTMLDivElement | null>(null);
  const hoverTimer = useRef(0);
  const [open, setOpen] = useState(false);
  const [focusOnOpen, setFocusOnOpen] = useState<FocusTarget>("first");
  const presence = usePresence<HTMLDivElement>(open);

  const openWith = (target: FocusTarget) => {
    window.clearTimeout(hoverTimer.current);
    setFocusOnOpen(target);
    setOpen(true);
  };

  // Moving to a sibling item in the parent menu closes this submenu.
  useEffect(() => {
    const item = itemRef.current;
    const parent = item?.closest<HTMLElement>('[role="menu"]');

    if (!open || item === null || parent === null || parent === undefined) {
      return;
    }

    const onFocusIn = (event: FocusEvent) => {
      const target = event.target;

      if (
        target instanceof Element &&
        target !== item &&
        target.closest('[role="menu"]') === parent
      ) {
        setOpen(false);
      }
    };

    parent.addEventListener("focusin", onFocusIn);

    return () => parent.removeEventListener("focusin", onFocusIn);
  }, [open]);

  useEffect(() => () => window.clearTimeout(hoverTimer.current), []);

  const onClose = (reason: CloseReason) => {
    setOpen(false);

    if (reason === "left") {
      itemRef.current?.focus({ preventScroll: true });
    } else if (reason !== "dismiss") {
      closeAll(reason === "escape" || reason === "select");
    }
  };

  return (
    <>
      <div
        ref={itemRef}
        role="menuitem"
        tabIndex={-1}
        className="menu-item"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-controls={presence.mounted ? id : undefined}
        data-label={label}
        onPointerMove={focusOnPointer}
        onPointerEnter={() => {
          window.clearTimeout(hoverTimer.current);
          hoverTimer.current = window.setTimeout(
            () => openWith("none"),
            readDurationMs("--duration-micro"),
          );
        }}
        onPointerLeave={() => window.clearTimeout(hoverTimer.current)}
        onClick={() => openWith("first")}
        onKeyDown={(event) => {
          if (event.key === "ArrowRight" || event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            event.stopPropagation();
            openWith("first");
          }
        }}
      >
        <span className="menu-item-icon">{icon === undefined ? null : <Icon name={icon} />}</span>
        <span className="menu-item-label">{label}</span>
        <Icon name="chevron-right" className="menu-item-chevron" />
      </div>
      {presence.mounted ? (
        <MenuSurface
          id={id}
          surfaceRef={presence.ref}
          anchorRef={itemRef}
          state={presence.state}
          placement="right-start"
          label={label}
          labelledBy={undefined}
          focusOnOpen={focusOnOpen}
          onClose={onClose}
          submenu
        >
          {children}
        </MenuSurface>
      ) : null}
    </>
  );
}
