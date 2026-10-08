import { LazyEmojiPicker, preloadEmojiPicker } from "../../lib/emoji/lazy-emoji-picker.tsx";
import type { RoomKind } from "../../store/model.ts";
import { Button } from "../../ui/button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Popover } from "../../ui/popover.tsx";
import { loadCustomIcons } from "../messages/commands.ts";
import { iconNameFor } from "./room-forms.ts";
import { loadEmoji, RoomGlyph } from "./room-glyph.tsx";

interface RoomIconButtonProps {
  readonly kind: RoomKind;
  readonly iconName: string | null;
  readonly onChange: (iconName: string | null) => void;
  readonly disabled?: boolean;
  readonly invalid?: boolean;
}

/**
 * The room's icon as a square well beside its name (Discord's server icon, Slack's channel
 * emoji): the kind's glyph until one is picked, a small "+" badge on hover, and the shared emoji
 * picker (emoji plus the workspace's icons) in a popover. "Remove icon" puts the kind's back.
 */
export function RoomIconButton({
  kind,
  iconName,
  onChange,
  disabled = false,
  invalid = false,
}: RoomIconButtonProps) {
  const label = iconName === null ? "Choose an icon" : `Change icon (:${iconName}:)`;

  if (disabled) {
    return (
      <span className="room-icon-well" data-disabled>
        <RoomGlyph kind={kind} iconName={iconName} size={22} />
      </span>
    );
  }

  return (
    <Popover
      label="Room icon"
      placement="bottom-start"
      trigger={(props) => (
        <button
          {...props}
          type="button"
          className="room-icon-well"
          aria-label={label}
          aria-invalid={invalid || undefined}
          data-empty={iconName === null || undefined}
          onPointerEnter={preloadEmojiPicker}
          onFocus={preloadEmojiPicker}
        >
          <RoomGlyph kind={kind} iconName={iconName} size={22} />
          <span className="room-icon-badge" aria-hidden="true">
            <Icon name="smile-plus" size={12} />
          </span>
        </button>
      )}
    >
      {(close) => (
        <div className="room-icon-popover">
          <LazyEmojiPicker
            loadCustomIcons={loadCustomIcons}
            onPick={(choice) => {
              close();
              void loadEmoji().then((emoji) => {
                const name = iconNameFor(choice.content, emoji);

                if (name !== null) onChange(name);
              });
            }}
          />
          {iconName === null ? null : (
            <div className="room-icon-popover-footer">
              <Button
                variant="ghost"
                size="sm"
                icon="x"
                onClick={() => {
                  close();
                  onChange(null);
                }}
              >
                Remove icon
              </Button>
            </div>
          )}
        </div>
      )}
    </Popover>
  );
}
