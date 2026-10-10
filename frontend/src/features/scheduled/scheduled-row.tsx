import type { KeyboardEvent } from "react";
import type { ConversationName } from "../../gen/ConversationName.ts";
import type { ScheduledMessage } from "../../gen/ScheduledMessage.ts";
import { formatFull } from "../../lib/time.ts";
import { Button } from "../../ui/button.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { MenuItem, MenuSeparator } from "../../ui/menu.tsx";
import { Tooltip } from "../../ui/tooltip.tsx";
import { sendAtLabel } from "../composer/schedule/presets.ts";
import { ConversationLabel, conversationText } from "../destinations/conversation-label.tsx";
import type { RowMotion } from "../destinations/list-motion.ts";
import { focusSiblingRow, ListRow } from "../destinations/list-row.tsx";
import type { MenuSource } from "../destinations/point-menu.tsx";
import { ScheduledFiles } from "./scheduled-files.tsx";
import { outcomeLabel, scheduledExcerpt, scheduledSection } from "./scheduled-format.ts";

/** What a scheduled row can ask for. */
export interface ScheduledRowHandlers {
  readonly onEdit: (item: ScheduledMessage) => void;
  readonly onReschedule: (item: ScheduledMessage) => void;
  readonly onSendNow: (item: ScheduledMessage) => void;
  readonly onCancel: (item: ScheduledMessage) => void;
  readonly onView: (item: ScheduledMessage) => void;
  readonly onMenu: (item: ScheduledMessage, source: MenuSource) => void;
}

/** Whether a row has a menu: a pending one, or a past one that went out (to view it). */
export function hasScheduledMenu(item: ScheduledMessage): boolean {
  return (
    scheduledSection(item) !== "past" || (item.state === "sent" && item.sentMessageId !== null)
  );
}

interface ScheduledMenuItemsProps {
  readonly item: ScheduledMessage;
  /** "Send now" is on its way for this one. */
  readonly sending: boolean;
  readonly handlers: ScheduledRowHandlers;
}

/** The row's menu (a long press, a right click, Shift+F10): the hover actions, or View message. */
export function ScheduledMenuItems({ item, sending, handlers }: ScheduledMenuItemsProps) {
  if (scheduledSection(item) === "past") {
    return (
      <MenuItem icon="arrow-up-right" onSelect={() => handlers.onView(item)}>
        View message
      </MenuItem>
    );
  }

  const busy = sending || item.state === "sending";

  return (
    <>
      <MenuItem icon="pencil" disabled={busy} onSelect={() => handlers.onEdit(item)}>
        Edit message
      </MenuItem>
      <MenuItem
        icon="send"
        disabled={busy || !item.sendable}
        onSelect={() => handlers.onSendNow(item)}
      >
        Send now
      </MenuItem>
      <MenuItem icon="calendar-clock" disabled={busy} onSelect={() => handlers.onReschedule(item)}>
        Reschedule
      </MenuItem>
      <MenuSeparator />
      <MenuItem icon="trash" tone="danger" disabled={busy} onSelect={() => handlers.onCancel(item)}>
        Cancel scheduled message
      </MenuItem>
    </>
  );
}

interface ScheduledRowProps {
  readonly item: ScheduledMessage;
  readonly conversation: ConversationName | null;
  readonly now: number;
  readonly motion: RowMotion;
  /** "Send now" is on its way for this one. */
  readonly sending: boolean;
  readonly handlers: ScheduledRowHandlers;
}

/** The past row's outcome line: sent (with "View message"), or why it wasn't. */
function Outcome({
  item,
  now,
  onView,
}: {
  readonly item: ScheduledMessage;
  readonly now: number;
  readonly onView: () => void;
}) {
  const sent = item.state === "sent";

  return (
    <span className="scheduled-outcome" data-tone={sent ? "success" : "danger"}>
      <Icon name={sent ? "check" : "ban"} size={12} />
      {outcomeLabel(item, now)}
      {sent && item.sentMessageId !== null ? (
        <>
          <span aria-hidden="true">·</span>
          <Button variant="link" size="sm" className="scheduled-view" onClick={onView}>
            View message
          </Button>
        </>
      ) : null}
    </span>
  );
}

/** The glyph beside a row: a clock while it waits, a warning when stranded, then the outcome. */
function glyphFor(item: ScheduledMessage) {
  const section = scheduledSection(item);

  if (section === "past") {
    return item.state === "sent" ? "send" : "ban";
  }

  return section === "stranded" ? "alert" : "clock";
}

/**
 * One scheduled message: where it goes, when, and its text. Upcoming and stranded rows open the
 * editor and offer send now, reschedule and cancel on hover, in their menu, and on touch screens
 * by a long press (the menu) or a swipe (cancel); a stranded one says why it can't go. Past rows
 * say whether it went (with "View message") or why not.
 */
export function ScheduledRow({
  item,
  conversation,
  now,
  motion,
  sending,
  handlers,
}: ScheduledRowProps) {
  const section = scheduledSection(item);
  const pending = section !== "past";
  const busy = sending || item.state === "sending";
  const when = sendAtLabel(new Date(item.sendAt), new Date(now));
  const viewable = item.state === "sent" && item.sentMessageId !== null;
  const menu = hasScheduledMenu(item);

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.target instanceof HTMLElement && event.target.closest(".list-row-bar") !== null) {
      return;
    }

    const plain = !event.metaKey && !event.ctrlKey && !event.altKey && !event.shiftKey;

    if (plain && (event.key === "ArrowDown" || event.key === "ArrowUp")) {
      event.preventDefault();
      focusSiblingRow(event.currentTarget, event.key === "ArrowDown" ? 1 : -1);
    } else if (pending && !busy && plain && (event.key === "Delete" || event.key === "Backspace")) {
      event.preventDefault();
      handlers.onCancel(item);
    } else if (menu && ((event.key === "F10" && event.shiftKey) || event.key === "ContextMenu")) {
      event.preventDefault();
      handlers.onMenu(item, event);
    }
  };

  const open = pending
    ? () => handlers.onEdit(item)
    : viewable
      ? () => handlers.onView(item)
      : null;

  return (
    <ListRow
      motion={motion}
      state={busy ? "busy" : section}
      onKeyDown={onKeyDown}
      onContextMenu={
        menu
          ? (event) => {
              event.preventDefault();
              handlers.onMenu(item, event);
            }
          : undefined
      }
      onLongPress={menu ? (x, y) => handlers.onMenu(item, { x, y }) : undefined}
      swipe={
        pending && !busy
          ? {
              label: "Cancel",
              icon: "trash",
              tone: "danger",
              onSwipe: () => handlers.onCancel(item),
            }
          : undefined
      }
      actions={
        pending ? (
          <>
            <Button
              variant="ghost"
              size="sm"
              icon="send"
              className="scheduled-send-now"
              loading={busy}
              loadingLabel="Sending"
              disabled={!item.sendable}
              onClick={() => handlers.onSendNow(item)}
            >
              Send now
            </Button>
            <IconButton
              icon="calendar-clock"
              label="Reschedule"
              size="sm"
              disabled={busy}
              onClick={() => handlers.onReschedule(item)}
            />
            <IconButton
              icon="pencil"
              label="Edit message"
              size="sm"
              disabled={busy}
              onClick={() => handlers.onEdit(item)}
            />
            <IconButton
              icon="trash"
              label="Cancel scheduled message"
              size="sm"
              className="scheduled-cancel"
              disabled={busy}
              onClick={() => handlers.onCancel(item)}
            />
          </>
        ) : undefined
      }
    >
      {open === null ? null : (
        <button
          type="button"
          className="list-row-open list-row-cover"
          aria-label={
            pending
              ? `Edit the message scheduled for ${when} in ${conversationText(conversation)}`
              : `View the message sent in ${conversationText(conversation)}`
          }
          disabled={pending && busy}
          onClick={open}
        />
      )}
      <div className="scheduled-row">
        <span className="scheduled-glyph" data-section={section} aria-hidden="true">
          <Icon name={glyphFor(item)} size={16} />
        </span>
        <div className="scheduled-main">
          {item.replyTarget === null ? null : (
            <p className="scheduled-reply">
              <Icon name="corner-up-left" size={12} className="scheduled-reply-icon" />
              Replying to {item.replyTarget.authorName}: {item.replyTarget.excerpt}
            </p>
          )}
          <div className="scheduled-line">
            <ConversationLabel conversation={conversation} />
            <Tooltip content={formatFull(item.sendAt)} describe={false}>
              <time
                className="scheduled-when list-row-time"
                dateTime={item.sendAt}
                data-section={section}
              >
                {busy ? "Sending…" : when}
              </time>
            </Tooltip>
          </div>
          <p className="scheduled-body">{scheduledExcerpt(item)}</p>
          <ScheduledFiles files={item.attachments} />
          {section === "stranded" ? (
            <span className="scheduled-outcome" data-tone="warning">
              <Icon name="alert" size={12} />
              <span className="scheduled-outcome-text">You can't post here any more</span>
            </span>
          ) : null}
          {section === "past" ? (
            <Outcome item={item} now={now} onView={() => handlers.onView(item)} />
          ) : null}
        </div>
      </div>
    </ListRow>
  );
}
