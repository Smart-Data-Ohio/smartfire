import type { KeyboardEvent, MouseEvent } from "react";
import type { ConversationName } from "../../gen/ConversationName.ts";
import type { SavedItem } from "../../gen/SavedItem.ts";
import { inlineMentions } from "../../lib/body-html.ts";
import { shortcutKeys } from "../../lib/shortcuts.ts";
import { formatFull } from "../../lib/time.ts";
import { SuccessCheck } from "../../motion/success-check.tsx";
import type { MessageDTO } from "../../store/model.ts";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Menu, MenuItem, MenuSeparator, SubMenu } from "../../ui/menu.tsx";
import { Tooltip } from "../../ui/tooltip.tsx";
import { sendAtLabel } from "../composer/schedule/presets.ts";
import { ConversationLabel, conversationText } from "../destinations/conversation-label.tsx";
import type { RowMotion } from "../destinations/list-motion.ts";
import { focusSiblingRow, ListRow } from "../destinations/list-row.tsx";
import { UNKNOWN_NAME, useUser } from "../people/people.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import { timeAgo } from "../threads/thread-format.ts";
import { ReminderItems } from "./reminder-items.tsx";

/** What a saved row can ask for. */
export interface SavedRowHandlers {
  readonly onOpen: (item: SavedItem) => void;
  readonly onToggleDone: (item: SavedItem) => void;
  /** A new reminder time, or `null` to clear it. */
  readonly onRemind: (item: SavedItem, at: Date | null) => void;
  readonly onCustomRemind: (item: SavedItem) => void;
  readonly onRemove: (item: SavedItem) => void;
  readonly onMenu: (
    item: SavedItem,
    event: MouseEvent<HTMLElement> | KeyboardEvent<HTMLElement>,
  ) => void;
}

interface SavedMenuItemsProps {
  readonly item: SavedItem;
  readonly handlers: SavedRowHandlers;
}

/** The row's menu, shared by its "More" button and its context menu. */
export function SavedMenuItems({ item, handlers }: SavedMenuItemsProps) {
  const done = item.status === "done";

  return (
    <>
      <MenuItem
        icon="arrow-up-right"
        shortcut={shortcutKeys("list-open")}
        onSelect={() => handlers.onOpen(item)}
      >
        Open message
      </MenuItem>
      <MenuItem
        icon={done ? "undo-2" : "circle-check"}
        shortcut={shortcutKeys("list-done")}
        onSelect={() => handlers.onToggleDone(item)}
      >
        {done ? "Move back to in progress" : "Mark as done"}
      </MenuItem>
      <SubMenu label={item.remindAt === null ? "Remind me" : "Change reminder"} icon="alarm-clock">
        <ReminderItems
          hasReminder={item.remindAt !== null}
          onRemind={(at) => handlers.onRemind(item, at)}
          onCustom={() => handlers.onCustomRemind(item)}
          onClear={() => handlers.onRemind(item, null)}
        />
      </SubMenu>
      <MenuSeparator />
      <MenuItem icon="trash" tone="danger" onSelect={() => handlers.onRemove(item)}>
        Remove from saved
      </MenuItem>
    </>
  );
}

/** "Reminds tomorrow at 9:00 AM", or "Reminded 2 hours ago" once it went out. */
export function reminderLabel(item: SavedItem, now: number): string | null {
  if (item.remindedAt !== null) {
    return `Reminded ${timeAgo(item.remindedAt, now)}`;
  }

  if (item.remindAt === null) {
    return null;
  }

  const when = sendAtLabel(new Date(item.remindAt), new Date(now));

  return `Reminds ${when.replace(/^(Today|Tomorrow)/, (word) => word.toLowerCase())}`;
}

interface SavedRowProps {
  readonly item: SavedItem;
  readonly message: MessageDTO | null;
  readonly conversation: ConversationName | null;
  readonly now: number;
  readonly motion: RowMotion;
  /** The viewer just marked it done here: the done chip draws its check in. */
  readonly celebrate: boolean;
  readonly handlers: SavedRowHandlers;
}

/**
 * One saved message: its author and conversation, the message itself (a few lines of it), its
 * attachment, the reminder and whether it's done. The row opens the message; hover (or focus)
 * shows done, remind and remove, plus the menu. Keys on a focused row: ↑/↓ move, Home/End jump,
 * ⏎ opens, E toggles done, Delete removes, Shift+F10 the menu.
 */
export function SavedRow({
  item,
  message,
  conversation,
  now,
  motion,
  celebrate,
  handlers,
}: SavedRowProps) {
  const author = useUser(message?.creatorId);
  const done = item.status === "done";
  const reminder = reminderLabel(item, now);
  const authorName = author?.name ?? UNKNOWN_NAME;

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.target instanceof HTMLElement && event.target.closest(".list-row-bar") !== null) {
      return;
    }

    const plain = !event.metaKey && !event.ctrlKey && !event.altKey && !event.shiftKey;

    if (plain && (event.key === "ArrowDown" || event.key === "ArrowUp")) {
      event.preventDefault();
      focusSiblingRow(event.currentTarget, event.key === "ArrowDown" ? 1 : -1);
    } else if (plain && (event.key === "e" || event.key === "E")) {
      event.preventDefault();
      handlers.onToggleDone(item);
    } else if (plain && (event.key === "Delete" || event.key === "Backspace")) {
      event.preventDefault();
      handlers.onRemove(item);
    } else if ((event.key === "F10" && event.shiftKey) || event.key === "ContextMenu") {
      event.preventDefault();
      handlers.onMenu(item, event);
    }
  };

  return (
    <ListRow
      motion={motion}
      state={done ? "done" : undefined}
      onKeyDown={onKeyDown}
      onContextMenu={(event) => {
        event.preventDefault();
        handlers.onMenu(item, event);
      }}
      actions={
        <>
          <IconButton
            icon={done ? "undo-2" : "circle-check"}
            label={done ? "Move back to in progress" : "Mark as done"}
            shortcut={shortcutKeys("list-done")}
            size="sm"
            onClick={() => handlers.onToggleDone(item)}
          />
          <Menu
            label="Remind me"
            placement="bottom-end"
            trigger={(props) => (
              <IconButton
                {...props}
                icon="alarm-clock"
                label={item.remindAt === null ? "Remind me" : "Change reminder"}
                size="sm"
              />
            )}
          >
            <ReminderItems
              hasReminder={item.remindAt !== null}
              onRemind={(at) => handlers.onRemind(item, at)}
              onCustom={() => handlers.onCustomRemind(item)}
              onClear={() => handlers.onRemind(item, null)}
            />
          </Menu>
          <IconButton
            icon="trash"
            label="Remove from saved"
            size="sm"
            className="saved-remove"
            onClick={() => handlers.onRemove(item)}
          />
        </>
      }
    >
      <button
        type="button"
        className="list-row-open list-row-cover"
        aria-label={`Open ${authorName}'s message in ${conversationText(conversation)}`}
        onClick={() => handlers.onOpen(item)}
      />
      <div className="saved-row" data-message-id={item.messageId}>
        <UserAvatar userId={message?.creatorId ?? 0} size={36} decorative />
        <div className="saved-main">
          <div className="saved-line">
            <span className="saved-author">{authorName}</span>
            <ConversationLabel conversation={conversation} />
            <Tooltip content={`Saved ${formatFull(item.createdAt)}`} describe={false}>
              <time className="saved-time list-row-time" dateTime={item.createdAt}>
                {timeAgo(item.createdAt, now)}
              </time>
            </Tooltip>
          </div>
          {message === null ? (
            <p className="saved-body text-faint">This message is no longer available.</p>
          ) : (
            <div
              className="saved-body message-body"
              // biome-ignore lint/security/noDangerouslySetInnerHtml: bodyHtml is the server's sanitizer output (crates/richtext), the HTML the classic views render
              dangerouslySetInnerHTML={{ __html: inlineMentions(message.bodyHtml) }}
            />
          )}
          {message?.attachment == null && reminder === null && !done ? null : (
            <div className="saved-chips">
              {message?.attachment == null ? null : (
                <span className="saved-chip">
                  <Icon name="paperclip" size={12} />
                  <span className="saved-chip-text">{message.attachment.filename}</span>
                </span>
              )}
              {reminder === null ? null : (
                <span
                  className="saved-chip"
                  data-tone={item.remindedAt === null ? "accent" : undefined}
                >
                  <Icon name="alarm-clock" size={12} />
                  {reminder}
                </span>
              )}
              {done ? (
                <span className="saved-chip" data-tone="success">
                  {celebrate ? <SuccessCheck size={12} /> : <Icon name="check" size={12} />}
                  Done
                </span>
              ) : null}
            </div>
          )}
        </div>
      </div>
    </ListRow>
  );
}
