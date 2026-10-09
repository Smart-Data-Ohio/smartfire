import { shortcutKeys } from "../../lib/shortcuts.ts";
import type { MessageDTO } from "../../store/model.ts";
import type { IconName } from "../../ui/icons/icon.tsx";
import { MenuItem, MenuSeparator } from "../../ui/menu.tsx";
import type { MessagePermissions } from "./permissions.ts";

/** Everything the message menu (and the hover bar, and the row keys) can ask the row to do. */
export type MenuCommand =
  | "reply"
  | "thread"
  | "react"
  | "boost"
  | "edit"
  | "copy-text"
  | "copy-link"
  | "pin"
  | "save"
  | "forward"
  | "unread"
  | "fizzy"
  | "delete";

/** One entry in the message menu, before it's rendered. */
export interface MenuEntry {
  readonly command: MenuCommand;
  readonly label: string;
  readonly icon: IconName;
  /** Empty when the entry has no single-key shortcut. */
  readonly shortcut: readonly string[];
  readonly danger: boolean;
}

interface MenuContext {
  readonly permissions: MessagePermissions;
  readonly pinned: boolean;
  readonly saved: boolean;
  readonly inThread: boolean;
}

function entry(
  command: MenuCommand,
  label: string,
  icon: IconName,
  shortcut: readonly string[] = [],
  danger = false,
): MenuEntry {
  return { command, label, icon, shortcut, danger };
}

/**
 * The menu's sections, in order, holding only what the viewer may do. Empty sections drop out,
 * so separators never double up.
 */
export function menuSections(context: MenuContext): readonly (readonly MenuEntry[])[] {
  const { permissions, pinned, saved, inThread } = context;
  const respond: MenuEntry[] = [];
  const share: MenuEntry[] = [];
  const keep: MenuEntry[] = [];
  const destroy: MenuEntry[] = [];

  if (permissions.reply) {
    respond.push(entry("reply", "Reply", "corner-up-left", shortcutKeys("message-reply")));
  }

  if (permissions.thread && !inThread) {
    respond.push(entry("thread", "Reply in thread", "thread", shortcutKeys("message-thread")));
  }

  if (permissions.react) {
    respond.push(entry("react", "Add reaction", "smile-plus", shortcutKeys("message-react")));
    respond.push(entry("boost", "Boost…", "rocket"));
  }

  if (permissions.edit) {
    share.push(entry("edit", "Edit message", "pencil", shortcutKeys("message-edit")));
  }

  share.push(entry("copy-text", "Copy text", "copy"));
  share.push(entry("copy-link", "Copy link", "link", shortcutKeys("message-link")));

  if (permissions.pin) {
    keep.push(
      pinned
        ? entry("pin", "Unpin from conversation", "pin-off", shortcutKeys("message-pin"))
        : entry("pin", "Pin to conversation", "pin", shortcutKeys("message-pin")),
    );
  }

  if (permissions.save) {
    keep.push(
      saved
        ? entry("save", "Remove from Saved", "bookmark-check", shortcutKeys("message-save"))
        : entry("save", "Save for later", "bookmark", shortcutKeys("message-save")),
    );
  }

  if (permissions.forward) {
    keep.push(entry("forward", "Forward…", "forward", shortcutKeys("message-forward")));
  }

  if (permissions.markUnread) {
    keep.push(entry("unread", "Mark unread", "message-dot"));
  }

  if (permissions.fizzy) {
    keep.push(entry("fizzy", "Create Fizzy card", "list-checks"));
  }

  if (permissions.remove) {
    destroy.push(entry("delete", "Delete message", "trash", shortcutKeys("message-delete"), true));
  }

  return [respond, share, keep, destroy].filter((section) => section.length > 0);
}

interface MessageMenuItemsProps {
  readonly message: MessageDTO;
  readonly permissions: MessagePermissions;
  readonly saved: boolean;
  readonly inThread: boolean;
  readonly onCommand: (command: MenuCommand) => void;
}

/** The message menu's items, for the right-click menu and the hover bar's "More". */
export function MessageMenuItems({
  message,
  permissions,
  saved,
  inThread,
  onCommand,
}: MessageMenuItemsProps) {
  const sections = menuSections({ permissions, pinned: message.pinned, saved, inThread });

  return sections.map((section, index) => (
    <MenuSection
      key={section[0]?.command ?? index}
      entries={section}
      separated={index > 0}
      onCommand={onCommand}
    />
  ));
}

interface MenuSectionProps {
  readonly entries: readonly MenuEntry[];
  readonly separated: boolean;
  readonly onCommand: (command: MenuCommand) => void;
}

function MenuSection({ entries, separated, onCommand }: MenuSectionProps) {
  return (
    <>
      {separated ? <MenuSeparator /> : null}
      {entries.map((item) =>
        item.danger ? (
          <MenuItem
            key={item.command}
            icon={item.icon}
            shortcut={item.shortcut}
            tone="danger"
            onSelect={() => onCommand(item.command)}
          >
            {item.label}
          </MenuItem>
        ) : (
          <MenuItem
            key={item.command}
            icon={item.icon}
            shortcut={item.shortcut}
            onSelect={() => onCommand(item.command)}
          >
            {item.label}
          </MenuItem>
        ),
      )}
    </>
  );
}
