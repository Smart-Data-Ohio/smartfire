import { useStore } from "../../store/store.ts";
import { Checkbox } from "../../ui/checkbox.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { snippet } from "../messages/message-content.tsx";
import { useViewerId } from "../messages/use-message.ts";
import { UNKNOWN_NAME, useUser } from "../people/people.ts";
import type { ReplyTarget } from "./reply-store.ts";

interface ReplyChipProps {
  readonly target: ReplyTarget;
  readonly onNotifyChange: (notify: boolean) => void;
  readonly onCancel: () => void;
}

/**
 * Above the composer's text while it replies: who it answers and the start of what they said,
 * "Notify author" (classic's toggle, on by default), and × (or Esc) to drop the reply. The toggle
 * hides on a reply to yourself, which never notifies anyone.
 */
export function ReplyChip({ target, onNotifyChange, onCancel }: ReplyChipProps) {
  const source = useStore((state) => state.messages[target.messageId]);
  const viewerId = useViewerId();
  const author = useUser(source?.creatorId);

  if (source === undefined) {
    return null;
  }

  const name = author?.name ?? UNKNOWN_NAME;
  const own = source.creatorId === viewerId;

  return (
    <section className="composer-reply enter-rise" aria-label={`Replying to ${name}`}>
      <Icon name="corner-up-left" size={14} className="composer-reply-icon" />
      <p className="composer-reply-copy">
        <span className="composer-reply-label" title={name}>
          Replying to <strong>{name}</strong>
        </span>
        <span className="composer-reply-text">{snippet(source)}</span>
      </p>
      {own ? null : (
        <span className="composer-reply-notify">
          <Checkbox
            checked={target.notify}
            onCheckedChange={onNotifyChange}
            label="Notify author"
          />
        </span>
      )}
      <IconButton
        icon="x"
        label="Cancel reply"
        shortcut={["Esc"]}
        size="sm"
        className="composer-reply-cancel"
        onMouseDown={(event) => event.preventDefault()}
        onClick={onCancel}
      />
    </section>
  );
}
