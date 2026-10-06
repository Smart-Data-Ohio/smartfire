import type { KeyboardEvent, MouseEvent } from "react";
import type { ActivityAction } from "../../gen/ActivityAction.ts";
import type { ActivityItem } from "../../gen/ActivityItem.ts";
import { shortcutKeys } from "../../lib/shortcuts.ts";
import { formatFull } from "../../lib/time.ts";
import { SuccessCheck } from "../../motion/success-check.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Menu, MenuItem, MenuSeparator } from "../../ui/menu.tsx";
import { Tooltip } from "../../ui/tooltip.tsx";
import type { RowMotion } from "../destinations/list-motion.ts";
import { focusSiblingRow, ListRow } from "../destinations/list-row.tsx";
import { useUser } from "../people/people.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import { timeAgo } from "../threads/thread-format.ts";
import { EVENT_ICON, EVENT_LABEL, EVENT_TONE, statusChip } from "./activity-format.ts";

interface ActivityMenuItemsProps {
  readonly item: ActivityItem;
  readonly onOpen: (item: ActivityItem) => void;
  readonly onAction: (item: ActivityItem, action: ActivityAction) => void;
}

/** The row's menu, shared by its "More" button and its context menu. */
export function ActivityMenuItems({ item, onOpen, onAction }: ActivityMenuItemsProps) {
  const unread = item.state === "unread";
  const handled = item.state === "handled";

  return (
    <>
      <MenuItem
        icon="arrow-up-right"
        shortcut={shortcutKeys("list-open")}
        onSelect={() => onOpen(item)}
      >
        Open
      </MenuItem>
      <MenuSeparator />
      <MenuItem
        icon={unread ? "check-check" : "circle-dot"}
        shortcut={shortcutKeys("list-read")}
        onSelect={() => onAction(item, unread ? "read" : "unread")}
      >
        {unread ? "Mark as read" : "Mark as unread"}
      </MenuItem>
      <MenuItem
        icon={handled ? "undo-2" : "circle-check"}
        shortcut={shortcutKeys("list-done")}
        onSelect={() => onAction(item, handled ? "unhandled" : "handled")}
      >
        {handled ? "Mark as not handled" : "Mark as handled"}
      </MenuItem>
    </>
  );
}

/** The person behind the item with its kind as a badge, or the kind alone as a tile. */
function ActivityGlyph({ item }: { readonly item: ActivityItem }) {
  const creatorId = item.source.creatorId;
  const icon = EVENT_ICON[item.eventType];
  const tone = EVENT_TONE[item.eventType];

  if (creatorId === null) {
    return (
      <span className="activity-glyph" data-tone={tone}>
        <span className="activity-tile">
          <Icon name={icon} size={18} />
        </span>
      </span>
    );
  }

  return (
    <span className="activity-glyph" data-tone={tone}>
      <UserAvatar userId={creatorId} size={36} decorative />
      <span className="activity-kind-badge">
        <Icon name={icon} size={11} />
      </span>
    </span>
  );
}

interface ActivityRowProps {
  readonly item: ActivityItem;
  readonly now: number;
  readonly motion: RowMotion;
  /** The viewer just handled it here: the handled chip draws its check in. */
  readonly celebrate: boolean;
  readonly onOpen: (item: ActivityItem) => void;
  readonly onAction: (item: ActivityItem, action: ActivityAction) => void;
  /** A context menu for the row, at the pointer or (from the keyboard) the row. */
  readonly onMenu: (
    item: ActivityItem,
    event: MouseEvent<HTMLElement> | KeyboardEvent<HTMLElement>,
  ) => void;
}

/**
 * One inbox entry, Slack's activity row: who (their avatar, badged with the kind), what kind and
 * where, the excerpt, and how long ago. Unread rows carry a dot and a heavier title; handled ones
 * a check. The whole row opens it; hover (or focus) shows read and handled toggles and a menu.
 * Keys on a focused row: ↑/↓ move, ⏎ opens, U toggles read, E toggles handled, Shift+F10 the menu.
 */
export function ActivityRow({
  item,
  now,
  motion,
  celebrate,
  onOpen,
  onAction,
  onMenu,
}: ActivityRowProps) {
  const source = item.source;
  const creator = useUser(source.creatorId ?? undefined);
  const unread = item.state === "unread";
  const handled = item.state === "handled";
  const label = EVENT_LABEL[item.eventType];
  const { title, occurredAt } = source;
  const chip = statusChip(item);
  const readAction: ActivityAction = unread ? "read" : "unread";
  const handledAction: ActivityAction = handled ? "unhandled" : "handled";

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.target instanceof HTMLElement && event.target.closest(".list-row-bar") !== null) {
      return;
    }

    const plain = !event.metaKey && !event.ctrlKey && !event.altKey;

    if ((event.key === "ArrowDown" || event.key === "ArrowUp") && plain && !event.shiftKey) {
      event.preventDefault();
      focusSiblingRow(event.currentTarget, event.key === "ArrowDown" ? 1 : -1);
    } else if (plain && !event.shiftKey && (event.key === "u" || event.key === "U")) {
      event.preventDefault();
      onAction(item, readAction);
    } else if (plain && !event.shiftKey && (event.key === "e" || event.key === "E")) {
      event.preventDefault();
      onAction(item, handledAction);
    } else if ((event.key === "F10" && event.shiftKey) || event.key === "ContextMenu") {
      event.preventDefault();
      onMenu(item, event);
    }
  };

  const summary = [
    unread ? "Unread" : handled ? "Handled" : null,
    label,
    title,
    creator?.name,
    timeAgo(occurredAt, now),
  ]
    .filter((part) => part !== null && part !== undefined)
    .join(", ");

  return (
    <ListRow
      motion={motion}
      state={item.state}
      onKeyDown={onKeyDown}
      onContextMenu={(event) => {
        event.preventDefault();
        onMenu(item, event);
      }}
      actions={
        <>
          <IconButton
            icon={unread ? "check-check" : "circle-dot"}
            label={unread ? "Mark as read" : "Mark as unread"}
            shortcut={shortcutKeys("list-read")}
            size="sm"
            onClick={() => onAction(item, readAction)}
          />
          <IconButton
            icon={handled ? "undo-2" : "circle-check"}
            label={handled ? "Mark as not handled" : "Mark as handled"}
            shortcut={shortcutKeys("list-done")}
            size="sm"
            className="activity-handle"
            onClick={() => onAction(item, handledAction)}
          />
          <Menu
            label="Activity actions"
            placement="bottom-end"
            trigger={(props) => (
              <IconButton {...props} icon="more" label="More actions" size="sm" />
            )}
          >
            <ActivityMenuItems item={item} onOpen={onOpen} onAction={onAction} />
          </Menu>
        </>
      }
    >
      <button
        type="button"
        className="list-row-open activity-row"
        aria-label={summary}
        onClick={() => onOpen(item)}
      >
        <span className="activity-dot" aria-hidden="true" />
        <ActivityGlyph item={item} />
        <span className="activity-main">
          <span className="activity-line">
            <span className="activity-kind" data-tone={EVENT_TONE[item.eventType]}>
              {label}
            </span>
            <span className="activity-sep" aria-hidden="true">
              ·
            </span>
            <span className="activity-title">{title}</span>
          </span>
          <span className="activity-body">
            {creator === undefined ? null : <span className="activity-author">{creator.name}</span>}
            {source.body}
          </span>
          {chip === null && !handled ? null : (
            <span className="activity-chips">
              {chip === null ? null : (
                <span className="activity-chip" data-status={chip.status}>
                  {chip.label}
                </span>
              )}
              {handled ? (
                <span className="activity-chip" data-status="handled">
                  {celebrate ? <SuccessCheck size={12} /> : <Icon name="check" size={12} />}
                  Handled
                </span>
              ) : null}
            </span>
          )}
        </span>
        <Tooltip content={formatFull(occurredAt)} describe={false}>
          <time className="activity-time" dateTime={occurredAt}>
            {timeAgo(occurredAt, now)}
          </time>
        </Tooltip>
      </button>
    </ListRow>
  );
}
