import { Link } from "@tanstack/react-router";
import type { ReactNode } from "react";
import type { MessageDTO } from "../../store/model.ts";
import { useStore } from "../../store/store.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import { UNKNOWN_NAME, useUser } from "../people/people.ts";
import { AttachmentView } from "./attachments.tsx";
import { plainText } from "./commands.ts";

/** How much of a quoted message the reply line shows before it trails off. */
const QUOTE_CHARS = 140;

function snippet(message: MessageDTO): string {
  const text = plainText(message).replace(/\s+/g, " ");

  if (text === "") {
    return message.attachment?.filename ?? "Attachment";
  }

  return text.length > QUOTE_CHARS ? `${text.slice(0, QUOTE_CHARS - 1)}…` : text;
}

/** "Pinned" and "Saved for later", above the header, as in Slack. */
export function MessageFlags({
  pinned,
  saved,
}: {
  readonly pinned: boolean;
  readonly saved: boolean;
}) {
  if (!pinned && !saved) {
    return null;
  }

  return (
    <div className="message-flags">
      {pinned ? (
        <span className="message-flag" data-flag="pinned">
          <Icon name="pin" size={12} />
          Pinned
        </span>
      ) : null}
      {saved ? (
        <span className="message-flag" data-flag="saved">
          <Icon name="bookmark-check" size={12} />
          Saved for later
        </span>
      ) : null}
    </div>
  );
}

/** The line above a reply: who it answers and the start of what they said. */
export function ReplyQuote({ message }: { readonly message: MessageDTO }) {
  const source = useStore((state) =>
    message.replyToMessageId === null ? undefined : state.messages[message.replyToMessageId],
  );

  const author = useUser(source?.creatorId);

  if (message.replyToMessageId === null) {
    return null;
  }

  if (source === undefined) {
    return (
      <div className="message-reply">
        <Icon name="corner-up-left" size={12} className="message-reply-icon" />
        <span className="message-reply-missing">Replying to a message</span>
      </div>
    );
  }

  return (
    <div className="message-reply">
      <Icon name="corner-up-left" size={12} className="message-reply-icon" />
      <Link
        to="/r/$roomId/m/$messageId"
        params={{ roomId: source.roomId, messageId: source.id }}
        className="message-reply-link"
        preload={false}
      >
        <span className="message-reply-author">{author?.name ?? UNKNOWN_NAME}</span>
        <span className="message-reply-text">{snippet(source)}</span>
      </Link>
    </div>
  );
}

function Body({ message }: { readonly message: MessageDTO }) {
  return (
    <div
      className="message-body"
      // biome-ignore lint/security/noDangerouslySetInnerHtml: bodyHtml is the server's sanitizer output (crates/richtext), the HTML the classic views render
      dangerouslySetInnerHTML={{ __html: message.bodyHtml }}
    />
  );
}

/** Where a forward came from, when the source message is in the store. */
function ForwardSource({ sourceId }: { readonly sourceId: number | null }) {
  const source = useStore((state) => (sourceId === null ? undefined : state.messages[sourceId]));
  const author = useUser(source?.creatorId);

  const roomName = useStore((state) =>
    source === undefined ? undefined : state.sidebar.rows[source.roomId]?.displayName,
  );

  if (source === undefined) {
    return null;
  }

  return (
    <>
      {roomName === undefined ? null : (
        <>
          {" from "}
          <Link
            to="/r/$roomId/m/$messageId"
            params={{ roomId: source.roomId, messageId: source.id }}
            className="message-forward-room"
            preload={false}
          >
            #{roomName}
          </Link>
        </>
      )}
      <span className="message-forward-author"> · {author?.name ?? UNKNOWN_NAME}</span>
    </>
  );
}

interface MessageContentProps {
  readonly message: MessageDTO;
  /** Shown at the end of the body's last line ("(edited)" on a continuation row). */
  readonly trailing?: ReactNode;
}

/**
 * The message's own content under the header: the body (or, for a forward, the forwarder's note
 * and the forwarded card), then its file.
 */
export function MessageContent({ message, trailing }: MessageContentProps) {
  if (message.forwardedAt === null) {
    return (
      <>
        <div className="message-body-row">
          <Body message={message} />
          {trailing}
        </div>
        {message.attachment === null ? null : <AttachmentView attachment={message.attachment} />}
      </>
    );
  }

  return (
    <>
      {message.forwardNote === null ? null : (
        <div className="message-body-row">
          <p className="message-forward-note">{message.forwardNote}</p>
          {trailing}
        </div>
      )}
      <div className="message-forward">
        <div className="message-forward-header">
          <Icon name="forward" size={12} />
          <span>
            Forwarded
            <ForwardSource sourceId={message.forwardedFromMessageId} />
          </span>
        </div>
        <div className="message-body-row">
          <Body message={message} />
          {message.forwardNote === null ? trailing : null}
        </div>
        {message.attachment === null ? null : <AttachmentView attachment={message.attachment} />}
      </div>
    </>
  );
}
