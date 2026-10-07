import { lazy, Suspense } from "react";
import { formatFull, formatTime } from "../../lib/time.ts";
import type { MessageDTO, PendingMessage } from "../../store/model.ts";
import { actions } from "../../sync/runtime.ts";
import { AgentThinking } from "../../ui/agent-thinking.tsx";
import { Button, Spinner } from "../../ui/button.tsx";
import { Tooltip } from "../../ui/tooltip.tsx";
import { PendingAttachmentView } from "../messages/attachments.tsx";
import { MessageContent, MessageFlags, ReplyQuote } from "../messages/message-content.tsx";
import { ReactionsRow } from "../messages/reactions.tsx";
import { useRowInteractions } from "../messages/row-interactions.tsx";
import { useViewerId } from "../messages/use-message.ts";
import { isAgent, UNKNOWN_NAME, useUser } from "../people/people.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import { ThreadIndicator } from "../threads/thread-indicator.tsx";
import { InlineMarkdown } from "./inline-markdown.tsx";
import "../messages/messages.css";

/** The inline editor, loaded the first time someone edits; the row keeps its text meanwhile. */
const MessageEditor = lazy(() =>
  import("../messages/message-editor.tsx").then((module) => ({ default: module.MessageEditor })),
);

interface HeaderProps {
  readonly creatorId: number;
  readonly createdAt: string;
  readonly edited: boolean;
}

/** Name, agent tag and time: the first row of a group. */
function MessageHeader({ creatorId, createdAt, edited }: HeaderProps) {
  const user = useUser(creatorId);

  return (
    <header className="message-header">
      <span className="message-author">{user?.name ?? UNKNOWN_NAME}</span>
      {isAgent(user) ? <span className="message-agent-tag">Agent</span> : null}
      <Tooltip content={formatFull(createdAt)} describe={false}>
        <time className="message-time tabular" dateTime={createdAt} tabIndex={-1}>
          {formatTime(createdAt)}
        </time>
      </Tooltip>
      {edited ? <span className="message-edited">(edited)</span> : null}
    </header>
  );
}

function Gutter({
  creatorId,
  createdAt,
  groupStart,
}: {
  readonly creatorId: number;
  readonly createdAt: string;
  readonly groupStart: boolean;
}) {
  return (
    <div className="message-gutter">
      {groupStart ? (
        <UserAvatar userId={creatorId} size={36} decorative />
      ) : (
        <time className="message-hover-time tabular" dateTime={createdAt}>
          {formatTime(createdAt)}
        </time>
      )}
    </div>
  );
}

interface MessageRowProps {
  readonly message: MessageDTO;
  readonly groupStart: boolean;
  readonly mentionsMe: boolean;
  readonly focused: boolean;
  readonly live: boolean;
  /** Set when the row renders inside a thread pane: no thread indicator, no "reply in thread". */
  readonly inThread?: boolean;
}

/**
 * A confirmed message: Slack's row anatomy on Discord's density. The body is the server's
 * sanitized HTML. Mentions of you get the amber bar; a permalinked row flashes; a message that
 * arrived live rises in (history never animates). Hovering (or focusing) shows the action bar;
 * right click, a long press or ⇧F10 opens the message menu; the row answers the message keys.
 */
export function MessageRow({
  message,
  groupStart,
  mentionsMe,
  focused,
  live,
  inThread = false,
}: MessageRowProps) {
  const creator = useUser(message.creatorId);
  const viewerId = useViewerId();
  const streaming = message.streaming && isAgent(creator);
  const row = useRowInteractions(message, inThread);
  const edited = message.editedAt !== null;
  const name = creator?.name ?? UNKNOWN_NAME;
  // A pinned message or a reply always shows who wrote it, under its flag or quote line.
  const header = groupStart || message.pinned || message.replyToMessageId !== null;

  const style =
    row.removingHeight === null ? undefined : { "--row-height": `${row.removingHeight}px` };

  return (
    <article
      ref={row.rowRef}
      className={`message${live ? " enter-rise" : ""}`}
      data-message-row
      data-message-id={message.id}
      data-group-start={header || undefined}
      data-mention={mentionsMe || undefined}
      data-focused={focused || undefined}
      data-system={message.systemNote || undefined}
      data-pinned={message.pinned || undefined}
      data-editing={row.editing || undefined}
      data-removing={row.removingHeight === null ? undefined : true}
      style={style}
      tabIndex={-1}
      aria-label={`${name}, ${formatTime(message.createdAt)}`}
      {...row.rowProps}
    >
      <Gutter creatorId={message.creatorId} createdAt={message.createdAt} groupStart={header} />
      <div className="message-main">
        <MessageFlags pinned={message.pinned} saved={row.saved} />
        <ReplyQuote message={message} />
        {header ? (
          <MessageHeader
            creatorId={message.creatorId}
            createdAt={message.createdAt}
            edited={edited}
          />
        ) : null}
        {streaming ? (
          <span className="message-streaming">
            <AgentThinking
              size={20}
              state="composing"
              label={`${creator?.name ?? "Agent"} is writing`}
            />
          </span>
        ) : null}
        {row.editing ? (
          <Suspense
            fallback={<MessageContent message={message} trailing={null} inThread={inThread} />}
          >
            <MessageEditor
              message={message}
              onClose={row.closeEditor}
              onRequestDelete={row.requestDelete}
            />
          </Suspense>
        ) : (
          <MessageContent
            message={message}
            inThread={inThread}
            trailing={!header && edited ? <span className="message-edited">(edited)</span> : null}
          />
        )}
        <ReactionsRow
          message={message}
          viewerId={viewerId}
          canReact={row.permissions.react}
          onAddReaction={row.onAddReaction}
        />
        {inThread || message.thread === null ? null : <ThreadIndicator message={message} />}
      </div>
      {row.bar}
      {row.overlays}
    </article>
  );
}

interface PendingRowProps {
  readonly pending: PendingMessage;
  readonly groupStart: boolean;
}

/**
 * A message on its way: 60 % opacity, a spinner only if it takes longer than 600 ms, and a
 * "Couldn't send" line with Retry and Delete when it fails.
 */
export function PendingRow({ pending, groupStart }: PendingRowProps) {
  const failed = pending.state === "failed";

  return (
    <article
      className="message enter-rise"
      data-group-start={groupStart || undefined}
      data-pending={pending.state}
      aria-label={failed ? "Message not sent" : "Sending message"}
    >
      <Gutter creatorId={pending.creatorId} createdAt={pending.createdAt} groupStart={groupStart} />
      <div className="message-main">
        {groupStart ? (
          <MessageHeader
            creatorId={pending.creatorId}
            createdAt={pending.createdAt}
            edited={false}
          />
        ) : null}
        <div className="message-body-row">
          <div className="message-body message-body-plain">
            {pending.markdownSource.trim() === "" ? null : (
              <InlineMarkdown source={pending.markdownSource} />
            )}
            {pending.attachment === null ? null : (
              <PendingAttachmentView attachment={pending.attachment} />
            )}
          </div>
          {failed ? null : (
            <span className="message-sending">
              <Spinner label="Sending" />
            </span>
          )}
        </div>
        {failed ? (
          <p className="message-failed t-input is-shaking" role="alert">
            <span>Couldn't send{pending.error === null ? "" : `: ${pending.error}`}</span>
            <Button variant="link" size="sm" onClick={() => actions.retry(pending.clientMessageId)}>
              Retry
            </Button>
            <Button
              variant="link"
              size="sm"
              onClick={() => actions.discard(pending.clientMessageId)}
            >
              Delete
            </Button>
          </p>
        ) : null}
      </div>
    </article>
  );
}
