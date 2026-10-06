import { formatFull, formatTime } from "../../lib/time.ts";
import type { MessageDTO, PendingMessage } from "../../store/model.ts";
import { actions } from "../../sync/runtime.ts";
import { AgentThinking } from "../../ui/agent-thinking.tsx";
import { Button, Spinner } from "../../ui/button.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { toast } from "../../ui/toast-store.ts";
import { Tooltip } from "../../ui/tooltip.tsx";
import { isAgent, UNKNOWN_NAME, useUser } from "../people/people.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import { InlineMarkdown } from "./inline-markdown.tsx";

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

function permalink(message: MessageDTO): string {
  return new URL(
    `${import.meta.env.BASE_URL}r/${message.roomId}/m/${message.id}`,
    window.location.origin,
  ).href;
}

/** The hover bar: commits after the hover-intent delay, leaves at once. More actions land in S2. */
function ActionBar({ message }: { readonly message: MessageDTO }) {
  const copy = () => {
    void navigator.clipboard.writeText(permalink(message)).then(
      () => toast({ title: "Link copied", tone: "success" }),
      () => toast({ title: "Couldn't copy the link", tone: "danger" }),
    );
  };

  return (
    <div className="message-actions" role="toolbar" aria-label="Message actions">
      <IconButton icon="link" label="Copy link" size="sm" onClick={copy} />
    </div>
  );
}

interface MessageRowProps {
  readonly message: MessageDTO;
  readonly groupStart: boolean;
  readonly mentionsMe: boolean;
  readonly focused: boolean;
  readonly live: boolean;
}

/**
 * A confirmed message: Slack's row anatomy on Discord's density. The body is the server's
 * sanitized HTML. Mentions of you get the amber bar; a permalinked row flashes; a message that
 * arrived live rises in (history never animates).
 */
export function MessageRow({ message, groupStart, mentionsMe, focused, live }: MessageRowProps) {
  const creator = useUser(message.creatorId);
  const streaming = message.streaming && isAgent(creator);

  return (
    <article
      className={`message${live ? " enter-rise" : ""}`}
      data-group-start={groupStart || undefined}
      data-mention={mentionsMe || undefined}
      data-focused={focused || undefined}
      data-system={message.systemNote || undefined}
      aria-label={`${creator?.name ?? UNKNOWN_NAME}, ${formatTime(message.createdAt)}`}
    >
      <Gutter creatorId={message.creatorId} createdAt={message.createdAt} groupStart={groupStart} />
      <div className="message-main">
        {groupStart ? (
          <MessageHeader
            creatorId={message.creatorId}
            createdAt={message.createdAt}
            edited={message.editedAt !== null}
          />
        ) : null}
        <div className="message-body-row">
          {streaming ? (
            <span className="message-streaming">
              <AgentThinking
                size={20}
                state="composing"
                label={`${creator?.name ?? "Agent"} is writing`}
              />
            </span>
          ) : null}
          <div
            className="message-body"
            // biome-ignore lint/security/noDangerouslySetInnerHtml: bodyHtml is the server's sanitizer output (crates/richtext), the HTML the classic views render
            dangerouslySetInnerHTML={{ __html: message.bodyHtml }}
          />
          {!groupStart && message.editedAt !== null ? (
            <span className="message-edited">(edited)</span>
          ) : null}
        </div>
      </div>
      <ActionBar message={message} />
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
            <InlineMarkdown source={pending.markdownSource} />
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
