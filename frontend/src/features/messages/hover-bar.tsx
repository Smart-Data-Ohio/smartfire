import { EmojiImage } from "../../lib/emoji/emoji-image.tsx";
import { preloadEmojiPicker } from "../../lib/emoji/lazy-emoji-picker.tsx";
import { type EmojiChoice, quickReactions, useRecentEmoji } from "../../lib/emoji/recent.ts";
import { shortcutKeys } from "../../lib/shortcuts.ts";
import { Button } from "../../ui/button.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Tooltip } from "../../ui/tooltip.tsx";
import type { MenuCommand } from "./message-menu.tsx";
import type { MessagePermissions } from "./permissions.ts";

interface HoverBarProps {
  readonly permissions: MessagePermissions;
  readonly saved: boolean;
  readonly inThread: boolean;
  /** The row's menu is open from this bar's "More" button. */
  readonly menuOpen: boolean;
  readonly onReact: (choice: EmojiChoice) => void;
  readonly onOpenPicker: (button: HTMLElement) => void;
  readonly onOpenMenu: (button: HTMLElement) => void;
  readonly onCommand: (command: MenuCommand) => void;
}

/**
 * The floating toolbar at a message's top right (Slack's anatomy): three quick reactions, add a
 * reaction, reply, reply in thread, forward, save, edit, and "More" for the full menu.
 */
export function HoverBar({
  permissions,
  saved,
  inThread,
  menuOpen,
  onReact,
  onOpenPicker,
  onOpenMenu,
  onCommand,
}: HoverBarProps) {
  const recent = useRecentEmoji();
  const quick = permissions.react ? quickReactions(recent, 3) : [];

  return (
    <div className="message-bar enter-bar" role="toolbar" aria-label="Message actions">
      {quick.map((choice) => (
        <Tooltip key={choice.content} content={`React with ${choice.title}`} describe={false}>
          <Button
            variant="icon"
            size="sm"
            className="message-bar-emoji"
            aria-label={`React with ${choice.title}`}
            onClick={() => onReact(choice)}
          >
            {choice.imageUrl === null ? (
              <span aria-hidden="true">{choice.content}</span>
            ) : (
              <EmojiImage src={choice.imageUrl} width={18} height={18} />
            )}
          </Button>
        </Tooltip>
      ))}
      {quick.length > 0 ? <span className="message-bar-separator" aria-hidden="true" /> : null}
      {permissions.react ? (
        <IconButton
          icon="smile-plus"
          size="sm"
          label="Add reaction"
          shortcut={shortcutKeys("message-react")}
          onPointerEnter={preloadEmojiPicker}
          onFocus={preloadEmojiPicker}
          onClick={(event) => onOpenPicker(event.currentTarget)}
        />
      ) : null}
      {permissions.reply ? (
        <IconButton
          icon="corner-up-left"
          size="sm"
          label="Reply"
          shortcut={shortcutKeys("message-reply")}
          onClick={() => onCommand("reply")}
        />
      ) : null}
      {permissions.thread && !inThread ? (
        <IconButton
          icon="thread"
          size="sm"
          label="Reply in thread"
          shortcut={shortcutKeys("message-thread")}
          onClick={() => onCommand("thread")}
        />
      ) : null}
      {permissions.forward ? (
        <IconButton
          icon="forward"
          size="sm"
          label="Forward"
          shortcut={shortcutKeys("message-forward")}
          onClick={() => onCommand("forward")}
        />
      ) : null}
      {permissions.save ? (
        <IconButton
          icon={saved ? "bookmark-check" : "bookmark"}
          size="sm"
          label={saved ? "Remove from Saved" : "Save for later"}
          shortcut={shortcutKeys("message-save")}
          aria-pressed={saved}
          className="message-bar-save"
          onClick={() => onCommand("save")}
        />
      ) : null}
      {permissions.edit ? (
        <IconButton
          icon="pencil"
          size="sm"
          label="Edit"
          shortcut={shortcutKeys("message-edit")}
          onClick={() => onCommand("edit")}
        />
      ) : null}
      <IconButton
        icon="more"
        size="sm"
        label="More actions"
        shortcut={shortcutKeys("message-menu")}
        aria-haspopup="menu"
        aria-expanded={menuOpen}
        onClick={(event) => onOpenMenu(event.currentTarget)}
      />
    </div>
  );
}
