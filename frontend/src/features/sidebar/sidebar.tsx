import { useParams } from "@tanstack/react-router";
import {
  type KeyboardEvent,
  type PointerEvent,
  type ReactNode,
  type RefObject,
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { setTheme, useAppearance } from "../../lib/appearance.ts";
import { shortcutKeys } from "../../lib/shortcuts.ts";
import type { RoomCategory, SidebarRow as Row } from "../../store/model.ts";
import { organizedSidebar } from "../../store/organize.ts";
import { useStore } from "../../store/store.ts";
import { Badge } from "../../ui/badge.tsx";
import { Button } from "../../ui/button.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { ariaKeyShortcuts, Kbd } from "../../ui/kbd.tsx";
import { Menu, MenuItem, MenuSeparator } from "../../ui/menu.tsx";
import { Skeleton, SkeletonReveal } from "../../ui/skeleton.tsx";
import { useAnnouncer } from "../destinations/live-region.tsx";
import { SidebarDestinations } from "../destinations/sidebar-destinations.tsx";
import { HuddleDock } from "../huddle/huddle-dock.tsx";
import { UNKNOWN_NAME } from "../people/people.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import { SidebarSearchButton } from "../search/sidebar-search-button.tsx";
import { UserMenu } from "../shell/user-menu.tsx";
import { useDestination } from "../shell/view-store.ts";
import { openOverlay } from "../switcher/overlay-store.ts";
import { CategoryNameField } from "./category-name-field.tsx";
import { DeleteCategoryDialog } from "./delete-category-dialog.tsx";
import * as commands from "./organize-commands.ts";
import {
  acceptsRoom,
  type DragItem,
  type DropTarget,
  FAVORITES_PLACEHOLDER,
  reorderedIds,
  sameTarget,
  slotForSection,
} from "./organize-model.ts";
import { RoomContextMenu, type RoomMenuRequest } from "./room-menu.tsx";
import {
  notificationLabel,
  peekingRows,
  type SidebarSection,
  sectionStatus,
  sectionUnread,
  sidebarSections,
} from "./sections.ts";
import {
  type DropEdge,
  type RowActions,
  RowActionsContext,
  RowGlyph,
  SidebarRow,
} from "./sidebar-row.tsx";
import { useFlip } from "./use-flip.ts";
import { type DragState, useSidebarDrag } from "./use-sidebar-drag.ts";
import "./sidebar.css";

const COLLAPSED_KEY = "smartfire.sidebar.collapsed";

function readCollapsed(): ReadonlySet<string> {
  try {
    const saved = localStorage.getItem(COLLAPSED_KEY);

    return new Set(saved === null ? [] : saved.split(","));
  } catch {
    return new Set();
  }
}

function writeCollapsed(keys: ReadonlySet<string>): void {
  try {
    localStorage.setItem(COLLAPSED_KEY, [...keys].join(","));
  } catch {
    // Unavailable storage only means the choice lasts for this page.
  }
}

/** What a section shows while something is dragged over the sidebar. */
interface SectionDrop {
  /** The room being dragged, if a room is. */
  readonly roomId: number | null;
  /** A room would land in this section as a whole (a category, Channels, an empty list). */
  readonly active: boolean;
  /** The dragged room can't go here. */
  readonly disabled: boolean;
  /** The row the Favourites drop line sits against, and on which side. */
  readonly line: { readonly roomId: number; readonly edge: DropEdge } | null;
  /** This section is the category being dragged. */
  readonly dragging: boolean;
  /** The line a category drop would land on: before or after this section. */
  readonly edge: DropEdge | null;
}

const NO_DROP: SectionDrop = {
  roomId: null,
  active: false,
  disabled: false,
  line: null,
  dragging: false,
  edge: null,
};

/** How `section` looks under the current drag. */
function sectionDrop(
  drag: DragState | null,
  section: SidebarSection,
  categories: readonly RoomCategory[],
): SectionDrop {
  if (drag === null) {
    return NO_DROP;
  }

  const { item, target } = drag;

  if (item.kind === "category") {
    const others = categories.filter((category) => category.id !== item.category.id);
    const dragging = section.category?.id === item.category.id;

    if (target?.kind !== "category" || section.category === null || dragging) {
      return { ...NO_DROP, dragging };
    }

    const before = others[target.index];
    const after = target.index === others.length ? others.at(-1) : undefined;

    const edge =
      before?.id === section.category.id
        ? "before"
        : after?.id === section.category.id
          ? "after"
          : null;

    return { ...NO_DROP, edge };
  }

  const roomId = item.row.room.id;
  const disabled = !acceptsRoom(section, item.row);

  if (target?.kind !== "room") {
    return { ...NO_DROP, roomId, disabled };
  }

  if (section.kind === "favorites") {
    if (target.slot.kind !== "favorite") {
      return { ...NO_DROP, roomId, disabled };
    }

    const others = section.rows.filter((row) => row.room.id !== roomId);
    const before = others[target.slot.index];
    const after = target.slot.index >= others.length ? others.at(-1) : undefined;

    const line =
      before === undefined
        ? after === undefined
          ? null
          : { roomId: after.room.id, edge: "after" as const }
        : { roomId: before.room.id, edge: "before" as const };

    return { ...NO_DROP, roomId, disabled, active: others.length === 0, line };
  }

  const here = slotForSection(section, item.row, 0);

  const active =
    here !== null && sameTarget(target, { kind: "room", slot: here } satisfies DropTarget);

  return { ...NO_DROP, roomId, disabled, active };
}

interface SectionProps {
  readonly section: SidebarSection;
  readonly open: boolean;
  readonly selectedRoomId: number | null;
  readonly onToggle: () => void;
  /** Buttons at the heading's end (Direct messages' "+", a category's menu). */
  readonly action?: ReactNode;
  /** In place of the title (and its buttons): the rename field. */
  readonly heading?: ReactNode;
  /** What an empty open section says, or offers. */
  readonly empty: ReactNode;
  readonly drop: SectionDrop;
  /** Dragging the heading reorders a category. */
  readonly onHeadingPointerDown?: ((event: PointerEvent<HTMLElement>) => void) | undefined;
}

/**
 * A collapsible group (the transitions.dev accordion). Collapsed, it still lists its unread rows
 * and the open conversation, as Slack does, so nothing new hides behind a chevron; its heading
 * brightens with the nub when anything inside is unread, and totals the notifications in red.
 */
function Section({
  section,
  open,
  selectedRoomId,
  onToggle,
  action,
  heading,
  empty,
  drop,
  onHeadingPointerDown,
}: SectionProps) {
  const id = useId();

  const peeking = open ? [] : peekingRows(section.rows, selectedRoomId);

  const folded = open ? { unread: false, count: 0 } : sectionUnread(section.rows);
  // A folded category says what it hides when its trigger takes focus. The status is `hidden` so
  // reading the page doesn't say it twice beside the badge; a description still reads it.
  const status = sectionStatus(folded);

  const edgeFor = (row: Row) => (drop.line?.roomId === row.room.id ? drop.line.edge : undefined);

  return (
    <div
      className="sidebar-section t-acc"
      data-open={open}
      data-unread={folded.unread || undefined}
      data-kind={section.kind}
      data-drop-section={section.key}
      data-flip={`section-${section.key}`}
      data-drop-active={drop.active || undefined}
      data-drop-disabled={drop.disabled || undefined}
      data-drop-edge={drop.edge ?? undefined}
      data-dragging={drop.dragging || undefined}
    >
      <div className="sidebar-section-heading">
        {heading ?? (
          <h2 className="sidebar-section-title">
            <button
              type="button"
              id={`${id}-trigger`}
              className="sidebar-section-trigger"
              aria-expanded={open}
              aria-controls={`${id}-panel`}
              aria-describedby={status === null ? undefined : `${id}-status`}
              onClick={onToggle}
              onPointerDown={onHeadingPointerDown}
            >
              <span className="t-acc-chevron">
                <Icon name="chevron-down" size={12} />
              </span>
              <span className="sidebar-section-name">{section.title}</span>
            </button>
          </h2>
        )}
        {heading === undefined ? action : null}
        {heading === undefined ? (
          <Badge count={folded.count} tone="danger" label={notificationLabel(folded.count)} />
        ) : null}
        {heading === undefined && status !== null ? (
          <span id={`${id}-status`} hidden>
            {status}
          </span>
        ) : null}
      </div>
      <section
        id={`${id}-panel`}
        className="t-acc-panel"
        aria-labelledby={heading === undefined ? `${id}-trigger` : undefined}
        aria-label={heading === undefined ? undefined : section.title}
        inert={!open}
      >
        <div className="t-acc-panel-inner">
          <ul className="sidebar-rows" data-list="main">
            {section.rows.map((row) => (
              <SidebarRow
                key={row.room.id}
                row={row}
                selected={row.room.id === selectedRoomId}
                main
                dropEdge={edgeFor(row)}
                dragging={row.room.id === drop.roomId}
              />
            ))}
            {section.rows.length === 0 ? <li className="sidebar-empty">{empty}</li> : null}
          </ul>
        </div>
      </section>
      {peeking.length > 0 ? (
        <ul className="sidebar-rows">
          {peeking.map((row) => (
            <SidebarRow key={row.room.id} row={row} selected={row.room.id === selectedRoomId} />
          ))}
        </ul>
      ) : null}
    </div>
  );
}

interface CategoryActionsProps {
  readonly category: RoomCategory;
  readonly index: number;
  readonly count: number;
  readonly onPickUp: KeyHandler;
  readonly onBlur: () => void;
  readonly onRename: () => void;
  readonly onMove: (index: number) => void;
  readonly onNew: () => void;
  readonly onDelete: () => void;
}

type KeyHandler = (event: KeyboardEvent<HTMLElement>) => void;

/** A category heading's end: its move handle (Space to pick up) and its menu. */
function CategoryActions({
  category,
  index,
  count,
  onPickUp,
  onBlur,
  onRename,
  onMove,
  onNew,
  onDelete,
}: CategoryActionsProps) {
  return (
    <>
      <IconButton
        icon="grip-vertical"
        label={`Move ${category.name}`}
        size="sm"
        className="sidebar-section-action sidebar-category-grip"
        data-drag-handle={`category-${category.id}`}
        aria-roledescription="draggable category"
        onKeyDown={onPickUp}
        onBlur={onBlur}
      />
      <Menu
        label={`${category.name} options`}
        placement="bottom-end"
        trigger={(props) => (
          <IconButton
            {...props}
            icon="more"
            label={`${category.name} options`}
            size="sm"
            className="sidebar-section-action"
          />
        )}
      >
        <MenuItem icon="pencil" onSelect={onRename}>
          Rename
        </MenuItem>
        <MenuItem disabled={index === 0} onSelect={() => onMove(index - 1)}>
          Move up
        </MenuItem>
        <MenuItem disabled={index === count - 1} onSelect={() => onMove(index + 1)}>
          Move down
        </MenuItem>
        <MenuItem icon="folder-plus" onSelect={onNew}>
          New category
        </MenuItem>
        <MenuSeparator />
        <MenuItem icon="trash" tone="danger" onSelect={onDelete}>
          Delete category
        </MenuItem>
      </Menu>
    </>
  );
}

/** The first-load placeholder: two sections' worth of rows, shaped like the real ones. */
function SidebarSkeleton() {
  const group = (key: string, widths: readonly number[], avatar: boolean) => (
    <div key={key} className="sidebar-skeleton-group">
      <Skeleton width={72} height={8} />
      {widths.map((width) => (
        // The widths are distinct within a group, so each one names its row.
        <span key={width} className="sidebar-skeleton-row">
          <Skeleton
            width={avatar ? 20 : 14}
            height={avatar ? 20 : 14}
            radius={avatar ? "md" : "sm"}
          />
          <Skeleton width={`${width}%`} height={9} />
        </span>
      ))}
    </div>
  );

  return (
    <div className="sidebar-skeleton">
      {group("channels", [62, 48, 70, 54, 40], false)}
      {group("direct", [58, 44, 66], true)}
    </div>
  );
}

/** The workspace header: the account's menu (shortcuts live there) and a new-message button. */
function WorkspaceHeader({
  title,
  onNewCategory,
}: {
  readonly title: string;
  readonly onNewCategory?: (() => void) | undefined;
}) {
  return (
    <header className="sidebar-header">
      <Menu
        label={`${title} menu`}
        trigger={(props) => (
          <Button
            {...props}
            variant="ghost"
            size="sm"
            trailingIcon="chevron-down"
            className="sidebar-workspace"
          >
            <span className="sidebar-workspace-name">{title}</span>
          </Button>
        )}
      >
        <MenuItem
          icon="square-pen"
          shortcut={shortcutKeys("new-direct")}
          onSelect={() => openOverlay("new-direct")}
        >
          New message
        </MenuItem>
        <MenuItem
          icon="search"
          shortcut={shortcutKeys("switcher")}
          onSelect={() => openOverlay("switcher")}
        >
          Jump to…
        </MenuItem>
        {onNewCategory === undefined ? null : (
          <MenuItem icon="folder-plus" onSelect={onNewCategory}>
            New category
          </MenuItem>
        )}
        <MenuSeparator />
        <MenuItem
          icon="keyboard"
          shortcut={shortcutKeys("shortcuts")}
          onSelect={() => openOverlay("shortcuts")}
        >
          Keyboard shortcuts
        </MenuItem>
      </Menu>
      <IconButton
        icon="square-pen"
        label="New message"
        shortcut={shortcutKeys("new-direct")}
        tooltipPlacement="bottom"
        className="sidebar-compose"
        onClick={() => openOverlay("new-direct")}
      />
      <SidebarSearchButton />
    </header>
  );
}

/** The search bar that opens the quick switcher, with its shortcut, as Slack's top bar does. */
function JumpButton() {
  return (
    <div className="sidebar-jump-wrap">
      <Button
        variant="secondary"
        size="sm"
        icon="search"
        className="sidebar-jump"
        aria-keyshortcuts={ariaKeyShortcuts(shortcutKeys("switcher"))}
        onClick={() => openOverlay("switcher")}
      >
        <span className="sidebar-jump-label">Jump to…</span>
        <Kbd keys={shortcutKeys("switcher")} className="sidebar-jump-kbd" />
      </Button>
    </div>
  );
}

const THEME_NEXT = { system: "light", light: "dark", dark: "system" } as const;

const THEME_ICON = { system: "monitor", light: "sun", dark: "moon" } as const;

/** Discord's user panel: who you are (it opens your menu), your presence, and the appearance switch. */
function YouPanel() {
  const me = useStore((state) => state.me);
  const bootUser = useStore((state) => state.boot?.user ?? null);
  const status = useStore((state) => (me === null ? null : (state.presence[me.user.id] ?? null)));
  const { theme } = useAppearance();
  const userId = me?.user.id ?? bootUser?.id;

  if (userId === undefined) {
    return null;
  }

  return (
    <footer className="sidebar-you">
      <UserMenu>
        <UserAvatar userId={userId} size={32} presence decorative />
        <span className="sidebar-you-text">
          <span className="sidebar-you-name">
            {me?.user.name ?? bootUser?.name ?? UNKNOWN_NAME}
          </span>
          <span className="sidebar-you-status">{status?.statusText ?? "Active"}</span>
        </span>
      </UserMenu>
      <IconButton
        icon={THEME_ICON[theme]}
        label={`Theme: ${theme}`}
        size="sm"
        onClick={() => setTheme(THEME_NEXT[theme])}
      />
    </footer>
  );
}

/** The floating copy of what is being dragged, following the pointer. */
function DragGhost({
  item,
  ghostRef,
}: {
  readonly item: DragItem;
  readonly ghostRef: RefObject<HTMLDivElement | null>;
}) {
  return (
    <div ref={ghostRef} className="sidebar-drag-ghost" data-kind={item.kind} aria-hidden="true">
      {item.kind === "room" ? (
        <>
          <RowGlyph row={item.row} />
          <span className="sidebar-row-name">{item.row.displayName}</span>
        </>
      ) : (
        <>
          <Icon name="chevron-down" size={12} />
          <span className="sidebar-row-name">{item.category.name}</span>
        </>
      )}
    </div>
  );
}

/**
 * The name field being shown: a new category (maybe for a room), or a rename. A new one keeps
 * what opened it, for focus to go back to when it's cancelled.
 */
type Editing =
  | { readonly kind: "create"; readonly row: Row | null; readonly opener: HTMLElement | null }
  | { readonly kind: "rename"; readonly categoryId: number }
  | null;

/** Where focus should go once the sidebar has rendered the change that calls for it. */
interface FocusRequest {
  /** The element, once it's there (`null` keeps the request for the next render). */
  readonly find: () => HTMLElement | null;
  /** Only when focus has fallen to the page meanwhile (nobody put it anywhere else). */
  readonly ifLost: boolean;
}

function focusIsLost(): boolean {
  const active = document.activeElement;

  return active === null || active === document.body;
}

/**
 * Hands focus over after the next render that has the element: a category's heading back from
 * its name field, a moved category's heading, a new category's heading once the server named it.
 */
function useFocusAfterRender(): (request: FocusRequest) => void {
  const pending = useRef<FocusRequest | null>(null);
  const [, rerender] = useState(0);

  useLayoutEffect(() => {
    const request = pending.current;
    const element = request?.find() ?? null;

    if (request === null || element === null) {
      return;
    }

    pending.current = null;

    if (!request.ifLost || focusIsLost()) {
      element.focus({ preventScroll: true });
    }
  });

  return (request) => {
    pending.current = request;
    rerender((count) => count + 1);
  };
}

/** The toggle button in a section's heading, by section key. */
function headingTrigger(container: HTMLElement | null, key: string): HTMLElement | null {
  return (
    container?.querySelector<HTMLElement>(
      `[data-drop-section="${key}"] .sidebar-section-trigger`,
    ) ?? null
  );
}

/** What opened a new category's field, when focus can go back to it (not a closing menu's item). */
function openerOf(active: Element | null): HTMLElement | null {
  return active instanceof HTMLElement && active.closest('[role="menu"]') === null ? active : null;
}

/**
 * The conversation list: the workspace header, then Favourites, your categories, Channels, Voice
 * and Direct messages, then your own panel. Rows and categories can be dragged into place (or
 * moved from their menus), categories created, renamed, folded and deleted, and each row's menu
 * sets its notification level.
 */
export function Sidebar() {
  const sidebar = useStore((state) => state.sidebar);
  const accountName = useStore((state) => state.boot?.account.name ?? null);
  const params = useParams({ strict: false });
  const [collapsed, setCollapsedKeys] = useState(readCollapsed);
  const [menu, setMenu] = useState<RoomMenuRequest | null>(null);
  const [editing, setEditing] = useState<Editing>(null);
  const [deleting, setDeleting] = useState<RoomCategory | null>(null);
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const menuIds = useRef(0);
  const hintId = useId();
  const selectedRoomId = params.roomId ?? null;
  const destination = useDestination();
  const view = organizedSidebar(sidebar);
  const categories = view.categories;
  const all = sidebarSections(sidebar);
  const flip = useFlip(scrollRef);
  const focusAfterRender = useFocusAfterRender();
  // A keyboard drag's steps interrupt: each one answers the key just pressed.
  const announcer = useAnnouncer("assertive");

  useEffect(() => commands.onBeforeOrganize(flip));

  const onDrop = (item: DragItem, target: DropTarget) => {
    if (item.kind === "room" && target.kind === "room") {
      commands.moveRoom(item.row, target.slot);
    } else if (item.kind === "category" && target.kind === "category") {
      commands.reorderCategories(reorderedIds(categories, item.category.id, target.index));
    }
  };

  // The drag reads the sections as shown, which depend on the drag itself (an empty Favourites
  // appears to take a drop while a room is dragged and nothing is starred yet).
  const shownRef = useRef<readonly SidebarSection[]>(all);

  const dnd = useSidebarDrag({
    containerRef: scrollRef,
    sectionsRef: shownRef,
    categories,
    onDrop,
    announce: announcer.announce,
  });

  const { drag } = dnd;

  const withPlaceholder =
    drag?.item.kind === "room" && !all.some((section) => section.kind === "favorites")
      ? [FAVORITES_PLACEHOLDER, ...all]
      : all;

  const sections =
    destination === "dms"
      ? withPlaceholder.filter((section) => section.key === "direct")
      : withPlaceholder;

  useLayoutEffect(() => {
    shownRef.current = sections;
  });

  const isOpen = (section: SidebarSection) =>
    section.category === null ? !collapsed.has(section.key) : !section.collapsed;

  const toggle = (section: SidebarSection) => {
    if (section.category !== null) {
      commands.setCollapsed(section.category, !section.collapsed);

      return;
    }

    const next = new Set(collapsed);

    if (next.has(section.key)) {
      next.delete(section.key);
    } else {
      next.add(section.key);
    }

    setCollapsedKeys(next);
    writeCollapsed(next);
  };

  const openMenu = (request: Omit<RoomMenuRequest, "id">) => {
    menuIds.current += 1;
    setMenu({ ...request, id: menuIds.current });
  };

  const rowActions: RowActions = {
    hintId,
    onPointerDown: (event, row) => dnd.onPointerDown(event, { kind: "room", row }),
    onKeyDown: (event, row) => dnd.onKeyDown(event, { kind: "room", row }),
    onBlur: dnd.onBlur,
    openMenuAt: (row, x, y) => {
      if (menu?.roomId !== row.room.id) {
        openMenu({
          roomId: row.room.id,
          x,
          y,
          width: 1,
          height: 1,
          placement: "bottom-start",
          keyboard: false,
        });
      }
    },
    openMenuFrom: (row, element, keyboard) => {
      const box = element.getBoundingClientRect();
      const fromRow = element.classList.contains("sidebar-row");

      openMenu({
        roomId: row.room.id,
        x: fromRow ? box.right - 32 : box.left,
        y: box.top,
        width: fromRow ? 28 : box.width,
        height: box.height,
        placement: "bottom-end",
        keyboard,
      });
    },
  };

  const menuRow = menu === null ? undefined : view.rows[menu.roomId];

  const newCategory = (row: Row | null = null) =>
    setEditing({ kind: "create", row, opener: openerOf(document.activeElement) });

  const focusHeading = (key: string, ifLost = false) =>
    focusAfterRender({ find: () => headingTrigger(scrollRef.current, key), ifLost });

  /** A cancelled new category: back to what opened it, else the room it was for, else Channels. */
  const focusAfterCancel = (opener: HTMLElement | null, row: Row | null) =>
    focusAfterRender({
      find: () => {
        if (opener?.isConnected === true) {
          return opener;
        }

        const roomRow =
          row === null
            ? null
            : scrollRef.current?.querySelector<HTMLElement>(
                `[data-room-id="${row.room.id}"] .sidebar-row`,
              );

        return roomRow ?? headingTrigger(scrollRef.current, "channels");
      },
      ifLost: false,
    });

  /**
   * Lands focus on a new category's heading: the draft's at once, the real one's once the server
   * has named it (the draft's heading goes with it, so only if focus is still nowhere else).
   */
  const create = (name: string, row: Row | null, refocus: boolean) => {
    const created = commands.createCategory(name, row);

    if (!refocus) {
      return;
    }

    focusAfterRender({
      find: () =>
        [
          ...(scrollRef.current?.querySelectorAll<HTMLElement>(
            '[data-drop-section^="category-"] .sidebar-section-trigger',
          ) ?? []),
        ].at(-1) ?? null,
      ifLost: false,
    });

    void created.then((category) => {
      if (category !== null) {
        focusHeading(`category-${category.id}`, true);
      }
    });
  };

  const newMessage = (
    <IconButton
      icon="plus"
      label="New message"
      shortcut={shortcutKeys("new-direct")}
      size="sm"
      className="sidebar-section-action"
      onClick={() => openOverlay("new-direct")}
    />
  );

  const newCategoryButton = (
    <IconButton
      icon="folder-plus"
      label="New category"
      size="sm"
      className="sidebar-section-action"
      onClick={() => newCategory()}
    />
  );

  const emptyFor = (section: SidebarSection) => {
    switch (section.kind) {
      case "direct":
        return (
          <Button
            variant="ghost"
            size="sm"
            icon="plus"
            className="sidebar-empty-action"
            onClick={() => openOverlay("new-direct")}
          >
            Start a conversation
          </Button>
        );
      case "favorites":
        return <span className="text-faint">Drop here to add a favourite</span>;
      case "category":
        return <span className="text-faint">Drag channels here</span>;
      default:
        return <span className="text-faint">No channels yet</span>;
    }
  };

  const actionFor = (section: SidebarSection) => {
    const { category } = section;

    if (category !== null) {
      const index = categories.findIndex((candidate) => candidate.id === category.id);

      return (
        <CategoryActions
          category={category}
          index={index}
          count={categories.length}
          onPickUp={(event) => dnd.onKeyDown(event, { kind: "category", category })}
          onBlur={dnd.onBlur}
          onRename={() => setEditing({ kind: "rename", categoryId: category.id })}
          onMove={(to) => {
            commands.reorderCategories(reorderedIds(categories, category.id, to));
            // The menu hands focus to its button, which moves with the section: the heading.
            focusHeading(section.key);
          }}
          onNew={() => newCategory()}
          onDelete={() => setDeleting(category)}
        />
      );
    }

    switch (section.kind) {
      case "channels":
        return newCategoryButton;
      case "direct":
        return newMessage;
      default:
        return undefined;
    }
  };

  const headingFor = (section: SidebarSection) => {
    const { category } = section;

    if (category === null || editing?.kind !== "rename" || editing.categoryId !== category.id) {
      return undefined;
    }

    return (
      <CategoryNameField
        label={`Rename ${category.name}`}
        initial={category.name}
        onSubmit={(name, refocus) => {
          commands.renameCategory(category, name);
          setEditing(null);

          if (refocus) {
            focusHeading(section.key);
          }
        }}
        onCancel={(refocus) => {
          setEditing(null);

          if (refocus) {
            focusHeading(section.key);
          }
        }}
      />
    );
  };

  const createField =
    editing?.kind === "create" ? (
      <div className="sidebar-section sidebar-new-category">
        <CategoryNameField
          label={
            editing.row === null
              ? "New category name"
              : `New category for ${editing.row.displayName}`
          }
          placeholder="New category"
          onSubmit={(name, refocus) => {
            create(name, editing.row, refocus);
            setEditing(null);
          }}
          onCancel={(refocus) => {
            setEditing(null);

            if (refocus) {
              focusAfterCancel(editing.opener, editing.row);
            }
          }}
        />
      </div>
    ) : null;

  const lastCategoryIndex = sections.findLastIndex((section) => section.kind === "category");

  const createAfter =
    lastCategoryIndex === -1
      ? sections.findIndex((section) => section.kind === "channels") - 1
      : lastCategoryIndex;

  return (
    <aside className="sidebar" aria-label="Conversations">
      <WorkspaceHeader
        title={destination === "dms" ? "Direct messages" : (accountName ?? "Smartfire")}
        onNewCategory={destination === "dms" ? undefined : () => newCategory()}
      />
      <JumpButton />
      <div
        className="sidebar-scroll"
        tabIndex={-1}
        ref={scrollRef}
        data-dragging={drag === null ? undefined : drag.item.kind}
      >
        <SidebarDestinations />
        {sidebar.status === "error" ? (
          <p className="sidebar-error text-meta">Couldn't load your conversations.</p>
        ) : (
          <SkeletonReveal loading={sidebar.status !== "ready"} skeleton={<SidebarSkeleton />}>
            <RowActionsContext value={rowActions}>
              {createAfter === -1 ? createField : null}
              {sections.map((section, index) => (
                <SectionWithField
                  key={section.key}
                  field={index === createAfter ? createField : null}
                >
                  <Section
                    section={section}
                    open={isOpen(section)}
                    selectedRoomId={selectedRoomId}
                    onToggle={() => toggle(section)}
                    action={actionFor(section)}
                    heading={headingFor(section)}
                    empty={emptyFor(section)}
                    drop={sectionDrop(drag, section, categories)}
                    onHeadingPointerDown={
                      section.category === null
                        ? undefined
                        : (event) => {
                            const { category } = section;

                            if (category !== null) {
                              dnd.onPointerDown(event, { kind: "category", category });
                            }
                          }
                    }
                  />
                </SectionWithField>
              ))}
            </RowActionsContext>
          </SkeletonReveal>
        )}
      </div>
      <p id={hintId} className="visually-hidden">
        Space picks it up to move it; Shift F10 opens its menu.
      </p>
      {announcer.region}
      {drag?.mode === "pointer" ? <DragGhost item={drag.item} ghostRef={dnd.ghostRef} /> : null}
      {menu !== null && menuRow !== undefined ? (
        <RoomContextMenu
          key={menu.id}
          request={menu}
          row={menuRow}
          categories={categories}
          onNewCategory={(row) => newCategory(row)}
          onClosed={(id) => setMenu((current) => (current?.id === id ? null : current))}
        />
      ) : null}
      <DeleteCategoryDialog
        category={deleting}
        roomCount={
          deleting === null
            ? 0
            : Object.values(view.rows).filter(
                (row) =>
                  row.membership.roomCategoryId === deleting.id &&
                  row.membership.favoritePosition === null &&
                  row.membership.involvement !== "invisible",
              ).length
        }
        onOpenChange={(open) => {
          if (!open) {
            setDeleting(null);
          }
        }}
        onConfirm={commands.deleteCategory}
      />
      <HuddleDock />
      <YouPanel />
    </aside>
  );
}

/** A section, with the new-category field after it when that's where the field goes. */
function SectionWithField({
  field,
  children,
}: {
  readonly field: ReactNode;
  readonly children: ReactNode;
}) {
  return (
    <>
      {children}
      {field}
    </>
  );
}
