import { type ReactNode, useState } from "react";
import { AgentAvatar } from "../../ui/agent-avatar.tsx";
import { AgentThinking } from "../../ui/agent-thinking.tsx";
import { Avatar, type PresenceStatus } from "../../ui/avatar.tsx";
import { Badge } from "../../ui/badge.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon, type IconName } from "../../ui/icons/icon.tsx";
import { Menu, MenuItem, MenuSeparator } from "../../ui/menu.tsx";
import { Tooltip } from "../../ui/tooltip.tsx";

/* ── Rail ─────────────────────────────────────────────────────────────────── */

function RailItem({
  label,
  active = false,
  unread = false,
  count = 0,
  children,
}: {
  readonly label: string;
  readonly active?: boolean;
  readonly unread?: boolean;
  readonly count?: number;
  readonly children: ReactNode;
}) {
  return (
    <div
      className="mock-rail-item"
      data-active={active || undefined}
      data-unread={unread || undefined}
    >
      <span className="mock-rail-pill" aria-hidden="true" />
      <Tooltip content={label} placement="right" describe={false}>
        <button
          type="button"
          className="mock-rail-button"
          aria-label={label}
          aria-current={active || undefined}
        >
          {children}
          <Badge count={count} floating label={`${count} mentions`} />
        </button>
      </Tooltip>
    </div>
  );
}

function Rail() {
  return (
    <nav className="mock-rail" aria-label="Workspaces">
      <RailItem label="Home" active>
        <Icon name="home" size={20} />
      </RailItem>
      <RailItem label="Direct messages" count={2}>
        <Icon name="dms" size={20} />
      </RailItem>
      <RailItem label="Activity" unread>
        <Icon name="bell" size={20} />
      </RailItem>
      <RailItem label="Boards">
        <Icon name="boards" size={20} />
      </RailItem>
      <span className="mock-rail-divider" aria-hidden="true" />
      <RailItem label="Smart Data Labs" unread count={5}>
        <span className="mock-workspace-tile">SD</span>
      </RailItem>
    </nav>
  );
}

/* ── Sidebar ──────────────────────────────────────────────────────────────── */

function ChannelRow({
  name,
  state,
  mentions = 0,
  icon = "hash",
}: {
  readonly name: string;
  readonly state?: "unread" | "muted" | "selected";
  readonly mentions?: number;
  readonly icon?: IconName;
}) {
  return (
    <li>
      <a
        href={`#${name}`}
        className="mock-row"
        data-state={state}
        aria-current={state === "selected" ? "page" : undefined}
        onClick={(event) => event.preventDefault()}
      >
        <Icon name={icon} size={16} className="mock-row-icon" />
        <span className="mock-row-name">{name}</span>
        {state === "muted" ? <Icon name="bell-off" size={14} className="mock-row-muted" /> : null}
        {mentions > 0 ? (
          <span className="mock-mention-pill">
            {mentions}
            <span className="visually-hidden"> mentions</span>
          </span>
        ) : null}
      </a>
    </li>
  );
}

function DmRow({
  name,
  presence,
  unread = false,
}: {
  readonly name: string;
  readonly presence: PresenceStatus;
  readonly unread?: boolean;
}) {
  return (
    <li>
      <a
        href={`#dm-${name}`}
        className="mock-row"
        data-state={unread ? "unread" : undefined}
        onClick={(event) => event.preventDefault()}
      >
        <Avatar name={name} size={20} presence={presence} decorative />
        <span className="mock-row-name">{name}</span>
      </a>
    </li>
  );
}

function Sidebar() {
  return (
    <aside className="mock-sidebar" aria-label="Channels">
      <header className="mock-sidebar-header">
        <button type="button" className="mock-workspace-name">
          Smart Data Labs
          <Icon name="chevron-down" size={14} />
        </button>
        <IconButton icon="pencil" label="New message" size="sm" shortcut={["⌘", "N"]} />
      </header>
      <div className="mock-sidebar-scroll">
        <ul className="mock-rows">
          <ChannelRow name="Threads" icon="thread" />
          <ChannelRow name="Mentions & reactions" icon="at" state="unread" />
          <ChannelRow name="Saved" icon="bookmark" />
        </ul>
        <p className="mock-section-label">Channels</p>
        <ul className="mock-rows">
          <ChannelRow name="general" state="unread" />
          <ChannelRow name="design" state="selected" />
          <ChannelRow name="frontend" mentions={2} state="unread" />
          <ChannelRow name="ops" />
          <ChannelRow name="random" state="muted" />
          <ChannelRow name="leadership" icon="lock" />
        </ul>
        <p className="mock-section-label">Direct messages</p>
        <ul className="mock-rows">
          <DmRow name="Grace Hopper" presence="online" unread />
          <DmRow name="Ada Lovelace" presence="away" />
          <DmRow name="Linus Torvalds" presence="dnd" />
          <DmRow name="Margaret Hamilton" presence="offline" />
        </ul>
      </div>
    </aside>
  );
}

/* ── Messages ─────────────────────────────────────────────────────────────── */

interface Reaction {
  readonly emoji: string;
  readonly label: string;
  readonly count: number;
  readonly mine: boolean;
}

function Reactions({ initial }: { readonly initial: readonly Reaction[] }) {
  const [reactions, setReactions] = useState(initial);

  const toggle = (emoji: string) => {
    setReactions(
      reactions.map((reaction) =>
        reaction.emoji === emoji
          ? { ...reaction, mine: !reaction.mine, count: reaction.count + (reaction.mine ? -1 : 1) }
          : reaction,
      ),
    );
  };

  return (
    <div className="mock-reactions">
      {reactions.map((reaction) => (
        <button
          key={reaction.emoji}
          type="button"
          className="mock-reaction"
          aria-pressed={reaction.mine}
          aria-label={`${reaction.label}: ${reaction.count}`}
          onClick={() => toggle(reaction.emoji)}
        >
          <span aria-hidden="true">{reaction.emoji}</span>
          <span className="tabular" aria-hidden="true">
            {reaction.count}
          </span>
        </button>
      ))}
      <IconButton icon="smile-plus" label="Add reaction" size="sm" className="mock-reaction-add" />
    </div>
  );
}

function ActionBar({ onThread }: { readonly onThread: () => void }) {
  return (
    <div className="mock-actions" role="toolbar" aria-label="Message actions">
      <IconButton icon="smile-plus" label="Add reaction" size="sm" />
      <IconButton
        icon="thread"
        label="Reply in thread"
        size="sm"
        shortcut={["T"]}
        onClick={onThread}
      />
      <IconButton icon="forward" label="Forward" size="sm" />
      <IconButton icon="bookmark" label="Save for later" size="sm" />
      <Menu
        placement="bottom-end"
        trigger={(props) => <IconButton {...props} icon="more" label="More actions" size="sm" />}
      >
        <MenuItem icon="link">Copy link</MenuItem>
        <MenuItem icon="bell">Remind me</MenuItem>
        <MenuItem icon="pin">Pin to channel</MenuItem>
        <MenuSeparator />
        <MenuItem icon="pencil">Edit message</MenuItem>
        <MenuItem icon="trash" tone="danger">
          Delete message
        </MenuItem>
      </Menu>
    </div>
  );
}

interface MessageProps {
  readonly author: string;
  readonly time: string;
  readonly agent?: boolean;
  readonly mention?: boolean;
  readonly continued?: boolean;
  readonly reactions?: readonly Reaction[];
  readonly onThread: () => void;
  readonly children: ReactNode;
}

function Message({
  author,
  time,
  agent = false,
  mention = false,
  continued = false,
  reactions,
  onThread,
  children,
}: MessageProps) {
  return (
    <article
      className="mock-message"
      data-mention={mention || undefined}
      data-continued={continued || undefined}
      aria-label={`${author}, ${time}`}
    >
      <div className="mock-message-gutter">
        {continued ? (
          <time className="mock-message-hover-time">{time.split(" ")[0]}</time>
        ) : agent ? (
          <AgentAvatar seed={author} name={author} size={36} decorative />
        ) : (
          <Avatar name={author} size={36} decorative />
        )}
      </div>
      <div className="mock-message-main">
        {continued ? null : (
          <header className="mock-message-header">
            <span className="mock-message-author">{author}</span>
            {agent ? <span className="mock-agent-tag">Agent</span> : null}
            <time className="mock-message-time">{time}</time>
          </header>
        )}
        <div className="mock-message-body">{children}</div>
        {reactions === undefined ? null : <Reactions initial={reactions} />}
      </div>
      <ActionBar onThread={onThread} />
    </article>
  );
}

function Chat({ onThread }: { readonly onThread: () => void }) {
  return (
    <div className="mock-messages">
      <div className="mock-day">
        <span>Today</span>
      </div>
      <Message author="Grace Hopper" time="9:41 AM" onThread={onThread}>
        <p>
          Pushed the new sidebar to staging. Unread channels are bold now, and muted ones fade back
          so they stop competing.
        </p>
      </Message>
      <Message author="Grace Hopper" time="9:41 AM" continued onThread={onThread}>
        <p>Tell me if the mention pill reads too loud in dark.</p>
      </Message>
      <Message
        author="Katherine Johnson"
        time="9:46 AM"
        mention
        onThread={onThread}
        reactions={[
          { emoji: "👍", label: "thumbs up", count: 4, mine: true },
          { emoji: "🎉", label: "party", count: 2, mine: false },
          { emoji: "👀", label: "eyes", count: 1, mine: false },
        ]}
      >
        <p>
          <span className="mock-mention">@Riel</span> the type scale is in. 15/22 for messages feels
          much closer to Slack.
        </p>
      </Message>
      <Message author="Scout" time="9:52 AM" agent onThread={onThread}>
        <p>
          Summary of #design since yesterday: sidebar shipped to staging, two open questions on the
          mention pill, and the type scale is settled.
        </p>
      </Message>
      <div className="mock-typing" aria-live="polite">
        <AgentThinking size={20} state="composing" label="Scout is typing" />
        <span>
          <strong>Scout</strong> is typing…
        </span>
      </div>
    </div>
  );
}

function ThreadPanel({ open, onClose }: { readonly open: boolean; readonly onClose: () => void }) {
  return (
    <aside className="mock-thread t-panel-slide" data-open={open} aria-label="Thread" inert={!open}>
      <header className="mock-pane-header">
        <div>
          <h2 className="mock-pane-title">Thread</h2>
          <p className="text-meta text-muted">#design</p>
        </div>
        <IconButton icon="x" label="Close thread" size="sm" onClick={onClose} shortcut={["Esc"]} />
      </header>
      <div className="mock-thread-body">
        <div className="mock-thread-root">
          <Avatar name="Grace Hopper" size={28} decorative />
          <p className="text-ui">Pushed the new sidebar to staging.</p>
        </div>
        <p className="mock-thread-count">2 replies</p>
        <div className="mock-thread-root">
          <Avatar name="Linus Torvalds" size={28} decorative />
          <p className="text-ui">Looks right. The muted rows could go one step fainter.</p>
        </div>
      </div>
    </aside>
  );
}

/** A miniature of the app: rail, sidebar (unread, mention pill, muted), chat rows and a thread. */
export function MockApp() {
  const [thread, setThread] = useState(false);

  return (
    <div className="mock-app" data-thread={thread || undefined}>
      <Rail />
      <Sidebar />
      <main className="mock-pane">
        <header className="mock-pane-header">
          <div className="mock-channel-title">
            <Icon name="hash" size={18} />
            <h2 className="mock-pane-title">design</h2>
          </div>
          <div className="mock-pane-tools">
            <IconButton icon="users" label="Members" size="sm" />
            <IconButton icon="phone" label="Start a huddle" size="sm" />
            <IconButton
              icon="thread"
              label={thread ? "Close thread" : "Open thread"}
              size="sm"
              aria-pressed={thread}
              onClick={() => setThread(!thread)}
            />
          </div>
        </header>
        <Chat onThread={() => setThread(true)} />
        <div className="mock-composer">
          <div className="mock-composer-box">
            <span className="text-faint">Message #design</span>
            <div className="mock-composer-tools">
              <IconButton icon="plus" label="Attach" size="sm" />
              <IconButton icon="at" label="Mention someone" size="sm" />
              <IconButton icon="send" label="Send" size="sm" shortcut={["⏎"]} />
            </div>
          </div>
        </div>
      </main>
      <ThreadPanel open={thread} onClose={() => setThread(false)} />
    </div>
  );
}
