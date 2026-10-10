import { useNavigate } from "@tanstack/react-router";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { RoomMuteDuration } from "../../gen/RoomMuteDuration.ts";
import type { Placement } from "../../lib/anchor.ts";
import { readDurationMs } from "../../motion/durations.ts";
import type { RoomCategory, SidebarRow } from "../../store/model.ts";
import { roomMuted, roomNotificationLevel } from "../../store/notification-preferences.ts";
import {
  canCategorize,
  defaultInvolvement,
  favoriteRows,
  organizedSidebar,
} from "../../store/organize.ts";
import type { State } from "../../store/state.ts";
import { useStore } from "../../store/store.ts";
import { settings as settingsActions } from "../../sync/settings.ts";
import {
  Menu,
  MenuGroup,
  MenuItem,
  MenuRadioItem,
  MenuSeparator,
  type MenuTriggerProps,
  SubMenu,
} from "../../ui/menu.tsx";
import { settingsOverState } from "../rooms/room-settings-host.tsx";
import { toastFailure } from "../settings/settings-parts.tsx";
import { markRead, moveRoom, setInvolvement, toggleFavorite } from "./organize-commands.ts";
import { involvementChoice, involvementChoices } from "./organize-model.ts";

/** A row's open menu: where it hangs (a viewport box) and how it was opened. */
export interface RoomMenuRequest {
  /** Bumps per request, so a closing menu can't clear the one that replaced it. */
  readonly id: number;
  readonly roomId: number;
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
  readonly placement: Placement;
  /** Opened from the keyboard: the menu lands on its first item. */
  readonly keyboard: boolean;
}

interface NotificationItemsProps {
  readonly row: Pick<SidebarRow, "room" | "displayName">;
  readonly level: SidebarRow["membership"]["involvement"];
}

/** The notification levels as radio items, the current one checked. */
export function NotificationItems({ row, level }: NotificationItemsProps) {
  const preferences = useStore((state) => state.sidebar.notificationPreferences);
  const effective = roomNotificationLevel(preferences, row.room.id, level);
  const inherited = preferences?.roomNotificationLevels[String(row.room.id)] === null;
  const [busy, setBusy] = useState(false);

  return (
    <>
      <MenuGroup label="Notify me about">
        <MenuRadioItem
          icon="settings"
          checked={inherited}
          description={`Account default: ${involvementChoice(preferences?.defaultNotificationLevel ?? "everything").label}`}
          onSelect={() => {
            if (busy) return;
            setBusy(true);
            void settingsActions
              .updateNotifications({ roomNotification: { roomId: row.room.id, level: null } })
              .catch((error: Error) => toastFailure("Couldn't change notifications", error))
              .finally(() => setBusy(false));
          }}
        >
          Use account default
        </MenuRadioItem>
        {involvementChoices(row).map((choice) => (
          <MenuRadioItem
            key={choice.level}
            icon={choice.icon}
            description={choice.description}
            checked={!inherited && choice.level === effective}
            onSelect={() => {
              if (inherited || choice.level !== effective) {
                setInvolvement(row.room.id, row.displayName, choice.level, effective);
              }
            }}
          >
            {choice.label}
          </MenuRadioItem>
        ))}
      </MenuGroup>
      <MenuSeparator />
      <RoomMuteItems roomId={row.room.id} />
    </>
  );
}

const MUTE_DURATIONS = [
  { duration: "minutes15", label: "15 minutes" },
  { duration: "hour1", label: "1 hour" },
  { duration: "hours8", label: "8 hours" },
  { duration: "hours24", label: "24 hours" },
  { duration: "forever", label: "Until I turn it back on" },
] as const satisfies readonly { readonly duration: RoomMuteDuration; readonly label: string }[];

export function RoomMuteItems({ roomId }: { readonly roomId: number }) {
  const preferences = useStore((state) => state.sidebar.notificationPreferences);
  const now = useStore((state) => state.sidebar.notificationClock ?? 0);
  const muted = roomMuted(preferences, roomId, now);
  const [busy, setBusy] = useState(false);

  const save = (duration: RoomMuteDuration) => {
    setBusy(true);
    void settingsActions
      .updateNotifications({ roomMute: { roomId, duration } })
      .catch((error: Error) => toastFailure("Couldn't change room mute", error))
      .finally(() => setBusy(false));
  };

  return (
    <>
      {muted ? (
        <MenuItem icon="bell" disabled={busy} onSelect={() => save("off")}>
          Unmute room
        </MenuItem>
      ) : null}
      <SubMenu label={muted ? "Change mute duration" : "Mute for"} icon="bell-off">
        {MUTE_DURATIONS.map(({ duration, label }) => (
          <MenuItem key={duration} disabled={busy} onSelect={() => save(duration)}>
            {label}
          </MenuItem>
        ))}
      </SubMenu>
    </>
  );
}

/** The favourites the sidebar shows, in order (one being hidden has left). */
function shownFavorites(state: State): readonly SidebarRow[] {
  return favoriteRows(organizedSidebar(state.sidebar)).filter(
    (row) => row.membership.involvement !== "invisible",
  );
}

/** Where a room sits among the shown favourites, and how many there are. */
interface FavoritePlace {
  /** -1 when the room isn't a favourite. */
  readonly index: number;
  readonly count: number;
}

function useFavoritePlace(roomId: number): FavoritePlace {
  const index = useStore((state) =>
    shownFavorites(state).findIndex((row) => row.room.id === roomId),
  );

  const count = useStore((state) => shownFavorites(state).length);

  return { index, count };
}

interface RoomMenuItemsProps {
  readonly row: SidebarRow;
  readonly categories: readonly RoomCategory[];
  /** "Move to → New category…": opens the name field, the room to move in once it exists. */
  readonly onNewCategory: (row: SidebarRow) => void;
}

/**
 * A conversation's menu: favourite (and, for a favourite, move up or down among them), move to a
 * category (channels only), notification level, mute, mark as read when there is something
 * unread, and the room's settings (not a direct message's: its header has rename and add people).
 */
export function RoomMenuItems({ row, categories, onNewCategory }: RoomMenuItemsProps) {
  const navigate = useNavigate();
  const preferences = useStore((state) => state.sidebar.notificationPreferences);
  const now = useStore((state) => state.sidebar.notificationClock ?? 0);
  const timedMute = roomMuted(preferences, row.room.id, now);
  const { favoritePosition, roomCategoryId, involvement, unreadAt } = row.membership;
  const starred = favoritePosition !== null;
  const favorite = useFavoritePlace(row.room.id);
  const { kind } = row.room;

  const openSettings = () => {
    // Closing settings steps back to whatever page this menu was opened on.
    void navigate({
      to: "/r/$roomId/settings",
      params: { roomId: row.room.id },
      state: settingsOverState(row.room.id),
    });
  };

  return (
    <>
      <MenuItem icon={starred ? "star-off" : "star"} onSelect={() => toggleFavorite(row)}>
        {starred ? "Remove from Favourites" : "Add to Favourites"}
      </MenuItem>
      {starred && favorite.index !== -1 && favorite.count > 1 ? (
        // Reordering without a drag (touch has none): the same move a drop makes.
        <>
          <MenuItem
            disabled={favorite.index === 0}
            onSelect={() => moveRoom(row, { kind: "favorite", index: favorite.index - 1 })}
          >
            Move up
          </MenuItem>
          <MenuItem
            disabled={favorite.index === favorite.count - 1}
            onSelect={() => moveRoom(row, { kind: "favorite", index: favorite.index + 1 })}
          >
            Move down
          </MenuItem>
        </>
      ) : null}
      {canCategorize(row) ? (
        <SubMenu label="Move to" icon="folder-input">
          <MenuRadioItem
            icon="star"
            checked={starred}
            onSelect={() => {
              if (!starred) {
                toggleFavorite(row);
              }
            }}
          >
            Favourites
          </MenuRadioItem>
          {categories.map((category) => (
            <MenuRadioItem
              key={category.id}
              icon="folder"
              checked={!starred && roomCategoryId === category.id}
              onSelect={() => moveRoom(row, { kind: "category", categoryId: category.id })}
            >
              {category.name}
            </MenuRadioItem>
          ))}
          <MenuRadioItem
            icon="hash"
            checked={!starred && roomCategoryId === null}
            onSelect={() => moveRoom(row, { kind: "channels" })}
          >
            Channels
          </MenuRadioItem>
          <MenuSeparator />
          <MenuItem icon="folder-plus" onSelect={() => onNewCategory(row)}>
            New category…
          </MenuItem>
        </SubMenu>
      ) : null}
      <SubMenu label="Notifications" icon={involvementChoice(involvement).icon}>
        <NotificationItems row={row} level={involvement} />
      </SubMenu>
      {involvement === "muted" || timedMute ? (
        <MenuItem
          icon="bell"
          onSelect={() => {
            if (timedMute) {
              void settingsActions
                .updateNotifications({ roomMute: { roomId: row.room.id, duration: "off" } })
                .catch((error: Error) => toastFailure("Couldn't unmute room", error));
            } else {
              setInvolvement(row.room.id, row.displayName, defaultInvolvement(row), involvement);
            }
          }}
        >
          Unmute
        </MenuItem>
      ) : null}
      {unreadAt === null ? null : (
        <>
          <MenuSeparator />
          <MenuItem icon="check-check" onSelect={() => markRead(row)}>
            Mark as read
          </MenuItem>
        </>
      )}
      {kind === "direct" ? null : (
        <>
          <MenuSeparator />
          <MenuItem icon="settings" onSelect={openSettings}>
            {kind === "open" || kind === "closed" ? "Channel settings" : "Settings"}
          </MenuItem>
        </>
      )}
    </>
  );
}

/**
 * Focuses a room's row: its main entry when it shows twice, any entry otherwise, and the
 * sidebar's list when the row is out of sight (moved into a folded category).
 */
function focusRoomRow(roomId: number): void {
  const row =
    document.querySelector<HTMLElement>(`[data-flip="room-${roomId}"] .sidebar-row`) ??
    document.querySelector<HTMLElement>(`[data-room-id="${roomId}"] .sidebar-row`);

  if (row !== null) {
    row.focus({ preventScroll: true });

    return;
  }

  const list = document.querySelector<HTMLElement>(".sidebar-scroll");

  list?.focus({ preventScroll: true });
}

interface AnchorProps {
  readonly trigger: MenuTriggerProps;
  readonly request: RoomMenuRequest;
  readonly onClosed: () => void;
}

/**
 * An invisible trigger where the menu should hang (the pointer, the row's "⋯" button or the row
 * itself). The design-system Menu opens from a trigger; this one clicks itself on mount, and
 * reports when the menu has finished closing.
 */
function Anchor({ trigger, request, onClosed }: AnchorProps) {
  const clicked = useRef(false);
  const seenOpen = useRef(false);
  const onClosedRef = useRef(onClosed);
  const expanded = trigger["aria-expanded"];
  const { ref, ...rest } = trigger;

  useLayoutEffect(() => {
    onClosedRef.current = onClosed;
  });

  // Opens once, at mount (the guard keeps a later render from clicking again).
  useLayoutEffect(() => {
    if (!clicked.current) {
      clicked.current = true;
      // A key-opened menu lands on its first item (`detail` 0); a pointer-opened one doesn't.
      ref.current?.dispatchEvent(
        new MouseEvent("click", {
          bubbles: true,
          cancelable: true,
          detail: request.keyboard ? 0 : 1,
        }),
      );
    }
  }, [ref, request.keyboard]);

  useEffect(() => {
    if (expanded) {
      seenOpen.current = true;

      return;
    }

    if (!seenOpen.current) {
      return;
    }

    // The menu hands focus back to this invisible anchor, which is about to go: put it on the
    // room's row instead, found again now in case the action moved it to another section.
    if (document.activeElement === ref.current) {
      focusRoomRow(request.roomId);
    }

    const timer = window.setTimeout(
      () => onClosedRef.current(),
      readDurationMs("--duration-small-exit"),
    );

    return () => window.clearTimeout(timer);
  }, [expanded, ref, request.roomId]);

  return (
    <button
      {...rest}
      ref={ref}
      type="button"
      tabIndex={-1}
      aria-hidden="true"
      className="sidebar-menu-anchor"
      style={{ left: request.x, top: request.y, width: request.width, height: request.height }}
    />
  );
}

interface RoomContextMenuProps extends Omit<RoomMenuItemsProps, "row"> {
  readonly request: RoomMenuRequest;
  readonly row: SidebarRow;
  readonly onClosed: (id: number) => void;
}

/** A row's menu, opened by a right click, a long press, its "⋯" button or Shift+F10. */
export function RoomContextMenu({
  request,
  row,
  categories,
  onNewCategory,
  onClosed,
}: RoomContextMenuProps) {
  return (
    <Menu
      label={`${row.displayName} options`}
      placement={request.placement}
      trigger={(trigger) => (
        <Anchor trigger={trigger} request={request} onClosed={() => onClosed(request.id)} />
      )}
    >
      <RoomMenuItems row={row} categories={categories} onNewCategory={onNewCategory} />
    </Menu>
  );
}
