import { Link } from "@tanstack/react-router";
import { type ReactNode, useState } from "react";
import type { MessageDTO } from "../../store/model.ts";
import { useStore } from "../../store/store.ts";
import { Button } from "../../ui/button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { CardSlot } from "../cards/card-slot.tsx";
import { AppTag } from "../people/agent-badge.tsx";
import { UNKNOWN_NAME, useIgnored, useUser } from "../people/people.ts";
import { AttachmentGallery } from "./attachments.tsx";
import { BodyHtml } from "./body-html.tsx";
import { plainText } from "./commands.ts";
import { fileSummary, messageFiles } from "./message-files.ts";
import { SoundMessage } from "./sound-message.tsx";
import { type DisplayedAuthor, displayedAuthor, forwardedAuthor } from "./webhook-attribution.ts";

/** How much of a quoted message the reply line shows before it trails off. */
const QUOTE_CHARS = 140;

/** One line of a message for a quote: its text, flattened and trimmed, or its file's name. */
export function snippet(message: MessageDTO): string {
  const text = plainText(message).replace(/\s+/g, " ");

  if (text === "") {
    return fileSummary(message);
  }

  const summary = messageFiles(message).length > 1 ? ` · ${fileSummary(message)}` : "";
  const chars = QUOTE_CHARS - summary.length;

  return `${text.length > chars ? `${text.slice(0, chars - 1)}…` : text}${summary}`;
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

/** Where a quote jumps to: the original on its timeline, a thread reply in its thread pane. */
function QuoteLink({
  source,
  children,
}: {
  readonly source: MessageDTO;
  readonly children: ReactNode;
}) {
  if (source.threadId !== null) {
    return (
      <Link
        to="/r/$roomId/t/$threadId"
        params={{ roomId: source.roomId, threadId: source.threadId }}
        search={{ m: source.id }}
        className="message-reply-link"
        preload={false}
      >
        {children}
      </Link>
    );
  }

  return (
    <Link
      to="/r/$roomId/m/$messageId"
      params={{ roomId: source.roomId, messageId: source.id }}
      className="message-reply-link"
      preload={false}
    >
      {children}
    </Link>
  );
}

/**
 * The line above a reply (or one being sent): who it answers and the start of what they said,
 * linking to the original.
 */
export function ReplyQuote({
  message,
}: {
  readonly message: Pick<MessageDTO, "replyToMessageId" | "replyTargetDeletedAt">;
}) {
  const { replyToMessageId, replyTargetDeletedAt } = message;

  const source = useStore((state) =>
    replyToMessageId === null ? undefined : state.messages[replyToMessageId],
  );

  const shown = source === undefined ? undefined : displayedAuthor(source);
  const user = useUser(shown?.kind === "person" ? shown.userId : undefined);
  const author = shown?.kind === "webhook" ? shown.name : (user?.name ?? UNKNOWN_NAME);

  if (replyTargetDeletedAt != null) {
    return (
      <div className="message-reply">
        <Icon name="corner-up-left" size={12} className="message-reply-icon" />
        <span className="message-reply-missing">Replying to a deleted message</span>
      </div>
    );
  }

  if (replyToMessageId === null) {
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
      <QuoteLink source={source}>
        <span className="message-reply-author">{author}</span>
        <span className="message-reply-text">{snippet(source)}</span>
      </QuoteLink>
    </div>
  );
}

function Body({ message }: { readonly message: MessageDTO }) {
  if (message.sound !== null) {
    return <SoundMessage sound={message.sound} />;
  }

  return <BodyHtml html={message.bodyHtml} className="message-body" />;
}

function Files({ message }: { readonly message: MessageDTO }) {
  return <AttachmentGallery attachments={messageFiles(message)} />;
}

/**
 * The forwarded block's "· author": a webhook's name with its APP tag, or a person's name. A
 * person comes from the loaded source only, so the store has its creator's row.
 */
function ForwardAuthor({ author }: { readonly author: DisplayedAuthor }) {
  const user = useUser(author.kind === "person" ? author.userId : undefined);

  if (author.kind === "webhook") {
    return (
      <>
        <span className="message-forward-author"> · {author.name}</span>
        <AppTag />
      </>
    );
  }

  return <span className="message-forward-author"> · {user?.name ?? UNKNOWN_NAME}</span>;
}

/** Where a forward came from (its room, when the source is in the store) and who wrote it. */
function ForwardSource({ copy }: { readonly copy: MessageDTO }) {
  const sourceId = copy.forwardedFromMessageId;
  const source = useStore((state) => (sourceId === null ? undefined : state.messages[sourceId]));
  const author = forwardedAuthor(copy, source);

  const roomName = useStore((state) =>
    source === undefined ? undefined : state.sidebar.rows[source.roomId]?.displayName,
  );

  return (
    <>
      {source === undefined || roomName === undefined ? null : (
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
      {author === null ? null : <ForwardAuthor author={author} />}
    </>
  );
}

interface MessageContentProps {
  readonly message: MessageDTO;
  /** Shown at the end of the body's last line ("(edited)" on a continuation row). */
  readonly trailing?: ReactNode;
  /** Rendered in a thread pane (the pane's root message heads its thread, cards and all). */
  readonly inThread?: boolean;
}

/**
 * The message's own content under the header: the body (or, for a forward, the forwarder's note
 * and the forwarded card), then its file, then its poll and cards.
 */
export function MessageContent({ message, trailing, inThread = false }: MessageContentProps) {
  // The thread pane's root message heads that thread; its replies head nothing.
  const heads = inThread && message.threadId === null ? (message.thread?.threadId ?? null) : null;
  const cards = <CardSlot message={message} threadId={heads} />;

  if (message.forwardedAt === null) {
    return (
      <>
        <div className="message-body-row">
          <Body message={message} />
          {trailing}
        </div>
        <Files message={message} />
        {cards}
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
            <ForwardSource copy={message} />
          </span>
        </div>
        <div className="message-body-row">
          <Body message={message} />
          {message.forwardNote === null ? trailing : null}
        </div>
        <Files message={message} />
        {cards}
      </div>
    </>
  );
}

/** "1 message from someone you ignore", "3 messages from someone you ignore". */
export function ignoredLabel(count: number): string {
  return `${count} ${count === 1 ? "message" : "messages"} from someone you ignore`;
}

/**
 * The line that stands in for messages from people you ignore, Discord style: how many, and Show
 * (Hide once shown). Showing is this screen's alone: nothing is saved, and a reload collapses
 * them again.
 */
export function IgnoredNotice({
  count,
  revealed,
  onToggle,
}: {
  readonly count: number;
  readonly revealed: boolean;
  readonly onToggle: () => void;
}) {
  return (
    <div className="message-ignored" data-revealed={revealed || undefined}>
      <span className="message-ignored-line">
        <Icon name="eye-off" size={14} className="message-ignored-icon" />
        <span className="message-ignored-text">{ignoredLabel(count)}</span>
        <span className="message-ignored-toggle">
          <span aria-hidden="true">—</span>
          <Button variant="link" size="sm" aria-expanded={revealed} onClick={onToggle}>
            {revealed ? "Hide" : "Show"}
          </Button>
        </span>
      </span>
    </div>
  );
}

/**
 * One message outside a timeline (a thread's parent): collapsed behind the line while you ignore
 * its author, as the store's ignore set has it (the message's own flag only until it loads).
 */
export function IgnoredGate({
  message,
  children,
}: {
  readonly message: MessageDTO;
  readonly children: ReactNode;
}) {
  const [shown, setShown] = useState(false);
  const ignored = useIgnored(message.creatorId, message.ignored);

  if (!ignored) {
    return children;
  }

  return (
    <>
      <IgnoredNotice count={1} revealed={shown} onToggle={() => setShown((open) => !open)} />
      {shown ? children : null}
    </>
  );
}

/** Which ignored runs a timeline shows, and the toggle its lines call. */
export interface IgnoredReveal {
  readonly revealed: ReadonlySet<number>;
  /** The focused message, unless the reader hid its run: a permalink opens the run it lands in. */
  readonly focusId: number | null;
  readonly toggle: (ids: readonly number[], open: boolean) => void;
}

/**
 * A timeline's shown ignored runs, by message id; local to the screen, never saved. `navigation`
 * names the navigation that brought the reader here (the history entry's key): each one that
 * lands on a permalink opens its run again, even after the reader hid it on an earlier visit.
 */
export function useIgnoredReveal(focusMessageId: number | null, navigation: string): IgnoredReveal {
  const [revealed, setRevealed] = useState<ReadonlySet<number>>(() => new Set());
  const target = `${navigation}:${focusMessageId ?? ""}`;
  const [seen, setSeen] = useState(target);
  const [focusHidden, setFocusHidden] = useState(false);

  // A new navigation or permalink: whatever the reader hid before, this one shows its target.
  if (seen !== target) {
    setSeen(target);
    setFocusHidden(false);
  }

  const toggle = (ids: readonly number[], open: boolean) => {
    setRevealed((current) => {
      const next = new Set(current);

      for (const id of ids) {
        if (open) next.add(id);
        else next.delete(id);
      }

      return next;
    });

    if (focusMessageId !== null && ids.includes(focusMessageId)) {
      setFocusHidden(!open);
    }
  };

  return {
    revealed,
    focusId: focusHidden && seen === target ? null : focusMessageId,
    toggle,
  };
}
