import { Link } from "@tanstack/react-router";
import type { ConversationName } from "../../gen/ConversationName.ts";
import { inlineMentions } from "../../lib/body-html.ts";
import { formatFull } from "../../lib/time.ts";
import type { MessageDTO } from "../../store/model.ts";
import { useStore } from "../../store/store.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import { fileIcon, formatBytes } from "../messages/format.ts";
import { UNKNOWN_NAME, useUser } from "../people/people.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import { ROOM_KIND_ICON } from "../room/room-icon.ts";
import { hitTime } from "./format.ts";
import { highlightHtml } from "./highlight.ts";

/** Where a conversation label goes: the room, or the thread open beside it. */
function ConversationLink({ name }: { readonly name: ConversationName }) {
  const roomLabel = (
    <>
      <Icon name={ROOM_KIND_ICON[name.roomKind]} size={14} className="search-conv-icon" />
      <span className="search-conv-room">{name.roomName}</span>
    </>
  );

  if (name.threadId === null) {
    return (
      <Link to="/r/$roomId" params={{ roomId: name.roomId }} className="search-conv">
        {roomLabel}
      </Link>
    );
  }

  return (
    <Link
      to="/r/$roomId/t/$threadId"
      params={{ roomId: name.roomId, threadId: name.threadId }}
      className="search-conv"
    >
      {roomLabel}
      <Icon name="chevron-right" size={12} className="search-conv-sep" />
      <Icon name="thread" size={14} className="search-conv-icon" />
      <span className="search-conv-thread">{name.threadName ?? "Thread"}</span>
    </Link>
  );
}

/** The conversation a run of hits is in, above the first of them. */
export function ConversationHeading({ name }: { readonly name: ConversationName | undefined }) {
  return (
    <div className="search-conv-heading">
      {name === undefined ? (
        <span className="search-conv">A conversation</span>
      ) : (
        <ConversationLink name={name} />
      )}
    </div>
  );
}

/** A compact line for the hit's file: a thumbnail or the file's icon, its name and size. */
function HitAttachment({ message }: { readonly message: MessageDTO }) {
  const { attachment } = message;
  const drive = message.cards.find((card) => card.kind === "drive");

  if (attachment !== null) {
    return (
      <span className="search-hit-file">
        {attachment.thumbnailUrl === null ? (
          <span className="search-hit-file-icon">
            <Icon name={fileIcon(attachment.contentType)} size={16} />
          </span>
        ) : (
          <img
            className="search-hit-thumb"
            src={attachment.thumbnailUrl}
            alt=""
            width={40}
            height={40}
            loading="lazy"
          />
        )}
        <span className="search-hit-file-name">{attachment.filename}</span>
        <span className="search-hit-file-size">{formatBytes(attachment.byteSize)}</span>
      </span>
    );
  }

  if (drive === undefined || drive.kind !== "drive") {
    return null;
  }

  // A Drive file's name isn't stored: the classic message calls it "Google Drive file" too.
  return (
    <a
      className="search-hit-file search-hit-drive"
      href={drive.data.url}
      target="_blank"
      rel="noreferrer"
    >
      <span className="search-hit-file-icon">
        <Icon name="file" size={16} />
      </span>
      <span className="search-hit-file-name">Google Drive file</span>
      <span className="search-hit-file-size">
        Open in Drive <Icon name="external-link" size={12} />
      </span>
    </a>
  );
}

interface HitRowProps {
  readonly hit: MessageDTO;
  readonly conversation: ConversationName | undefined;
  /** The searched words, for marking. */
  readonly terms: readonly string[];
  readonly now: number;
}

/**
 * One matching message, Slack's search card: avatar, author and day-stamped time, the body with
 * the searched words marked, and its file. The author's name is the card's link (stretched over
 * the whole card) to the message in its room, or in its thread for a reply; links in the body
 * stay clickable above it. A newer copy in the live store wins; a deleted message drops out.
 */
export function HitRow({ hit, conversation, terms, now }: HitRowProps) {
  const live = useStore((state) => state.messages[hit.id]);
  const deleted = useStore((state) => state.tombstones[hit.id] !== undefined);
  const message = live !== undefined && live.updatedAt >= hit.updatedAt ? live : hit;
  const author = useUser(message.creatorId);
  const name = author?.name ?? UNKNOWN_NAME;

  if (deleted) {
    return null;
  }

  const when = hitTime(message.createdAt, now);
  const place = conversation === undefined ? "" : ` in ${conversation.roomName}`;
  const label = `${name}${place}, ${when}`;

  const html =
    message.bodyHtml === "" ? "" : highlightHtml(inlineMentions(message.bodyHtml), terms);

  return (
    <article className="search-hit" data-search-hit>
      <UserAvatar userId={message.creatorId} size={36} decorative />
      <div className="search-hit-main">
        <div className="search-hit-head">
          {message.threadId === null ? (
            <Link
              to="/r/$roomId/m/$messageId"
              params={{ roomId: message.roomId, messageId: message.id }}
              className="search-hit-link"
              aria-label={label}
              data-search-hit-link
            >
              {name}
            </Link>
          ) : (
            <Link
              to="/r/$roomId/t/$threadId"
              params={{ roomId: message.roomId, threadId: message.threadId }}
              search={{ m: message.id }}
              className="search-hit-link"
              aria-label={label}
              data-search-hit-link
            >
              {name}
            </Link>
          )}
          <time
            className="search-hit-time"
            dateTime={message.createdAt}
            title={formatFull(message.createdAt)}
          >
            {when}
          </time>
          {message.editedAt === null ? null : <span className="search-hit-edited">(edited)</span>}
        </div>
        {html === "" ? null : (
          <div
            className="search-hit-body message-body"
            // biome-ignore lint/security/noDangerouslySetInnerHtml: the server's sanitized body; the marks are added as DOM nodes, never as markup text
            dangerouslySetInnerHTML={{ __html: html }}
          />
        )}
        <HitAttachment message={message} />
      </div>
      <span className="search-hit-jump" aria-hidden="true">
        Jump
        <Icon name="arrow-up-right" size={14} />
      </span>
    </article>
  );
}
