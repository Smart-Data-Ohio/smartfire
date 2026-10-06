import { LazyEmojiPicker, preloadEmojiPicker } from "../../lib/emoji/lazy-emoji-picker.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Popover } from "../../ui/popover.tsx";
import { loadCustomIcons } from "../messages/commands.ts";

/**
 * The composer's emoji button: opens the shared emoji picker (its own lazy chunk, fetched when the
 * pointer comes near) above the toolbar, and inserts the choice at the caret: the character for
 * an emoji, `:name:` for a workspace or brand icon.
 */
export function ComposerEmojiButton({ onInsert }: { readonly onInsert: (text: string) => void }) {
  return (
    <Popover
      label="Insert emoji"
      placement="top-start"
      trigger={(props) => (
        <IconButton
          {...props}
          icon="smile"
          label="Emoji"
          size="sm"
          onPointerEnter={preloadEmojiPicker}
          onFocus={preloadEmojiPicker}
          onMouseDown={(event) => event.preventDefault()}
        />
      )}
    >
      {(close) => (
        <LazyEmojiPicker
          loadCustomIcons={loadCustomIcons}
          onPick={(choice) => {
            // Close first: the popover hands focus to its trigger, then the insert puts it back
            // in the text box at the new caret.
            close();
            onInsert(choice.content);
          }}
        />
      )}
    </Popover>
  );
}
