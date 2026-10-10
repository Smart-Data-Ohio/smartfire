import { useNavigate, useSearch } from "@tanstack/react-router";
import { useEffect, useId, useRef, useState } from "react";
import { parseBoardSearch } from "../../lib/board-search.ts";
import type { ThreadPermissions } from "../../store/model.ts";
import { useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Menu, MenuItem, MenuSeparator } from "../../ui/menu.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import { PostWork } from "../boards/post-work.tsx";
import { Composer } from "../composer/composer.tsx";
import { PaneFrame, RoomName } from "../panes/pane-frame.tsx";
import { PaneError } from "../panes/pane-states.tsx";
import { HandoffArrival } from "../work/handoff-arrival.tsx";
import { LinksArrival } from "../work/links-arrival.tsx";
import { TrackAsWorkItem, WorkBar, WorkLive } from "../work/work-bar.tsx";
import { THREAD_STATUS_LABEL, threadTitle } from "./thread-format.ts";
import { foreignThreadHref } from "./thread-target.ts";
import { ThreadTimeline } from "./thread-timeline.tsx";

const DELETED = "This thread was deleted.";

/** `ChannelThread::AUTO_ARCHIVE_OPTIONS`, in minutes, as the picker offers them. */
const AUTO_ARCHIVE_OPTIONS = [
  { minutes: 60, label: "1 hour" },
  { minutes: 1440, label: "1 day" },
  { minutes: 4320, label: "3 days" },
  { minutes: 10080, label: "1 week" },
] as const;

const DEFAULT_AUTO_ARCHIVE_MINUTES = 4320;

/** "1 week", for a duration the picker knows. */
function autoArchiveLabel(minutes: number): string {
  return (
    AUTO_ARCHIVE_OPTIONS.find((option) => option.minutes === minutes)?.label ?? `${minutes} minutes`
  );
}

/** "thread", or "post" in a board, whose threads are posts. */
type Noun = "thread" | "post";

function useNoun(roomId: number): Noun {
  return useStore((state) =>
    state.rooms[roomId]?.detail?.room.kind === "board" ? "post" : "thread",
  );
}

function capitalized(noun: Noun): string {
  return noun === "post" ? "Post" : "Thread";
}

/** The absolute URL of a thread, for "Copy link". */
export function threadLink(roomId: number, threadId: number): string {
  return new URL(`${import.meta.env.BASE_URL}r/${roomId}/t/${threadId}`, window.location.origin)
    .href;
}

function copyLink(roomId: number, threadId: number): void {
  void navigator.clipboard.writeText(threadLink(roomId, threadId)).then(
    () => toast({ title: "Link copied", tone: "success" }),
    () => toast({ title: "Couldn't copy the link", tone: "danger" }),
  );
}

/** Runs a thread write and says so when it fails. */
function attempt(write: Promise<void>, failure: string): Promise<boolean> {
  return write.then(
    () => true,
    (error: Error) => {
      toast({ title: failure, description: error.message, tone: "danger" });

      return false;
    },
  );
}

/** "# general" in the header, with the thread's status when it isn't active. */
function ThreadSubtitle({
  roomId,
  threadId,
}: {
  readonly roomId: number;
  readonly threadId: number;
}) {
  const status = useStore((state) => state.threads[threadId]?.status ?? "active");

  return (
    <>
      <RoomName roomId={roomId} />
      {status === "active" ? null : (
        <span className="thread-status" data-status={status}>
          <Icon name={status === "locked" ? "lock" : "archive"} size={12} />
          {THREAD_STATUS_LABEL[status]}
        </span>
      )}
    </>
  );
}

/** Follow (every reply notifies) or unfollow (mentions only). */
function FollowButton({ threadId }: { readonly threadId: number }) {
  const following = useStore(
    (state) => state.threadMemberships[threadId]?.involvement === "everything",
  );

  const [busy, setBusy] = useState(false);

  const toggle = () => {
    setBusy(true);
    void attempt(
      actions.threads.follow(threadId, following ? "mentions" : "everything"),
      following ? "Couldn't unfollow the thread" : "Couldn't follow the thread",
    ).then(() => setBusy(false));
  };

  return (
    <Button
      variant="ghost"
      size="sm"
      icon={following ? "bell" : "bell-off"}
      className="thread-follow"
      data-following={following || undefined}
      aria-pressed={following}
      loading={busy}
      onClick={toggle}
    >
      {following ? "Following" : "Follow"}
    </Button>
  );
}

interface ThreadMenuProps {
  readonly roomId: number;
  readonly threadId: number;
  readonly permissions: ThreadPermissions | null;
  readonly noun: Noun;
  readonly onRename: () => void;
  readonly onAutoArchive: () => void;
  readonly onDelete: () => void;
}

/**
 * Copy link, then what the viewer's permissions allow: rename, set how long an idle thread stays
 * open, close or reopen, lock or unlock, delete, and track as work.
 */
function ThreadMenu({
  roomId,
  threadId,
  permissions,
  noun,
  onRename,
  onAutoArchive,
  onDelete,
}: ThreadMenuProps) {
  const status = useStore((state) => state.threads[threadId]?.status ?? "active");

  const autoArchive = useStore(
    (state) => state.threads[threadId]?.autoArchiveAfterMinutes ?? DEFAULT_AUTO_ARCHIVE_MINUTES,
  );

  const update = (body: Parameters<typeof actions.threads.update>[1], failure: string) =>
    void attempt(actions.threads.update(threadId, body), failure);

  const canClose = permissions?.canClose === true && status === "active";
  const canReopen = permissions?.canReopen === true && status === "closed";
  const canLock = permissions?.canLock === true && status !== "locked";
  const canUnlock = permissions?.canUnlock === true && status === "locked";

  const canDelete = permissions?.canDelete === true;
  const canConvert = permissions?.canConvertWork === true;

  const moderates =
    permissions?.canRename === true ||
    canClose ||
    canReopen ||
    canLock ||
    canUnlock ||
    canDelete ||
    canConvert;

  return (
    <Menu
      placement="bottom-end"
      label={`${capitalized(noun)} actions`}
      trigger={(props) => (
        <IconButton
          {...props}
          icon="more"
          label={`${capitalized(noun)} actions`}
          tooltipPlacement="bottom"
        />
      )}
    >
      <MenuItem icon="link" onSelect={() => copyLink(roomId, threadId)}>
        Copy link
      </MenuItem>
      {moderates ? <MenuSeparator /> : null}
      {permissions?.canRename === true ? (
        <MenuItem icon="pencil" onSelect={onRename}>
          Rename {noun}…
        </MenuItem>
      ) : null}
      {permissions?.canRename === true && noun !== "post" ? (
        <MenuItem icon="timer" detail={autoArchiveLabel(autoArchive)} onSelect={onAutoArchive}>
          Auto-archive after…
        </MenuItem>
      ) : null}
      {canClose ? (
        <MenuItem
          icon="archive"
          onSelect={() =>
            update(
              { name: null, autoArchiveAfterMinutes: null, status: "closed" },
              `Couldn't close the ${noun}`,
            )
          }
        >
          Close {noun}
        </MenuItem>
      ) : null}
      {canReopen ? (
        <MenuItem
          icon="rotate-ccw"
          onSelect={() =>
            update(
              { name: null, autoArchiveAfterMinutes: null, status: "active" },
              `Couldn't reopen the ${noun}`,
            )
          }
        >
          Reopen {noun}
        </MenuItem>
      ) : null}
      {canLock ? (
        <MenuItem
          icon="lock"
          onSelect={() =>
            update(
              { name: null, autoArchiveAfterMinutes: null, status: "locked" },
              `Couldn't lock the ${noun}`,
            )
          }
        >
          Lock {noun}
        </MenuItem>
      ) : null}
      {canUnlock ? (
        <MenuItem
          icon="lock-open"
          onSelect={() =>
            update(
              { name: null, autoArchiveAfterMinutes: null, status: "active" },
              `Couldn't unlock the ${noun}`,
            )
          }
        >
          Unlock {noun}
        </MenuItem>
      ) : null}
      {canDelete ? <MenuSeparator /> : null}
      {canDelete ? (
        <MenuItem icon="trash" tone="danger" onSelect={onDelete}>
          Delete {noun}…
        </MenuItem>
      ) : null}
      {canConvert ? <TrackAsWorkItem threadId={threadId} /> : null}
    </Menu>
  );
}

/** Renames the thread: 1 to 100 characters. */
function RenameDialog({
  threadId,
  noun,
  open,
  onOpenChange,
}: {
  readonly threadId: number;
  readonly noun: Noun;
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
}) {
  const current = useStore((state) => state.threads[threadId]?.name ?? "");
  const [name, setName] = useState(current);
  // The name as this opening found it: someone renaming it meanwhile doesn't make the field dirty.
  const [opening, setOpening] = useState(current);
  const [error, setError] = useState<string | undefined>(undefined);
  const [attempts, setAttempts] = useState(0);
  const [saving, setSaving] = useState(false);
  const [wasOpen, setWasOpen] = useState(open);

  if (open !== wasOpen) {
    setWasOpen(open);

    if (open) {
      setName(current);
      setOpening(current);
      setError(undefined);
    }
  }

  const save = () => {
    const trimmed = name.trim();

    if (trimmed === "" || trimmed.length > 100) {
      setError(trimmed === "" ? `Give the ${noun} a name.` : "Keep it to 100 characters.");
      setAttempts((count) => count + 1);

      return;
    }

    setSaving(true);
    actions.threads
      .update(threadId, {
        name: trimmed,
        autoArchiveAfterMinutes: null,
        status: null,
      })
      .then(
        () => {
          setSaving(false);
          onOpenChange(false);
        },
        (failure: Error) => {
          setSaving(false);
          setError(failure.message);
          setAttempts((count) => count + 1);
        },
      );
  };

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title={`Rename ${noun}`}
      size="sm"
      dirty={name !== opening}
      footer={
        <>
          <Button variant="secondary" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button variant="primary" loading={saving} onClick={save}>
            Save
          </Button>
        </>
      }
    >
      <form
        onSubmit={(event) => {
          event.preventDefault();
          save();
        }}
      >
        <TextField
          label="Name"
          value={name}
          maxLength={100}
          error={error}
          attempt={attempts}
          data-autofocus
          onChange={(event) => setName(event.target.value)}
        />
      </form>
    </Dialog>
  );
}

/** Sets how long an idle thread stays open before it reads as closed. */
function AutoArchiveDialog({
  threadId,
  open,
  onOpenChange,
}: {
  readonly threadId: number;
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
}) {
  const fieldId = useId();

  const current = useStore(
    (state) => state.threads[threadId]?.autoArchiveAfterMinutes ?? DEFAULT_AUTO_ARCHIVE_MINUTES,
  );

  const [minutes, setMinutes] = useState(current);
  // The duration as this opening found it: someone changing it meanwhile doesn't make it dirty.
  const [opening, setOpening] = useState(current);
  const [error, setError] = useState<string | undefined>(undefined);
  const [saving, setSaving] = useState(false);
  const [wasOpen, setWasOpen] = useState(open);

  if (open !== wasOpen) {
    setWasOpen(open);

    if (open) {
      setMinutes(current);
      setOpening(current);
      setError(undefined);
    }
  }

  const save = () => {
    setSaving(true);
    actions.threads
      .update(threadId, {
        name: null,
        autoArchiveAfterMinutes: minutes,
        status: null,
      })
      .then(
        () => {
          setSaving(false);
          onOpenChange(false);
        },
        (failure: Error) => {
          setSaving(false);
          setError(failure.message);
        },
      );
  };

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title="Auto-archive"
      size="sm"
      dirty={minutes !== opening}
      footer={
        <>
          <Button variant="secondary" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button variant="primary" loading={saving} onClick={save}>
            Save
          </Button>
        </>
      }
    >
      <form
        onSubmit={(event) => {
          event.preventDefault();
          save();
        }}
      >
        <div className={`field t-input-wrap${error === undefined ? "" : " is-error"}`}>
          <label className="field-label" htmlFor={fieldId}>
            Auto-archive after
          </label>
          <select
            id={fieldId}
            className={`input t-input${error === undefined ? "" : " is-error"}`}
            value={minutes}
            aria-invalid={error === undefined ? undefined : true}
            aria-describedby={error === undefined ? undefined : `${fieldId}-error`}
            data-autofocus
            onChange={(event) => setMinutes(Number(event.target.value))}
          >
            {AUTO_ARCHIVE_OPTIONS.map((option) => (
              <option key={option.minutes} value={option.minutes}>
                {option.label}
              </option>
            ))}
          </select>
          <p id={`${fieldId}-error`} className="field-error t-error-msg" aria-live="polite">
            {error ?? ""}
          </p>
        </div>
      </form>
    </Dialog>
  );
}

/**
 * A locked thread refuses every reply, moderators' too, so the composer gives way to a disabled
 * bar that says why; whoever may unlock it gets the button right there.
 */
function LockedComposer({ threadId, noun }: { readonly threadId: number; readonly noun: Noun }) {
  const canUnlock = useStore(
    (state) => state.threadPanes[threadId]?.permissions?.canUnlock === true,
  );

  const [busy, setBusy] = useState(false);

  const unlock = () => {
    setBusy(true);
    void attempt(
      actions.threads.update(threadId, {
        name: null,
        autoArchiveAfterMinutes: null,
        status: "active",
      }),
      `Couldn't unlock the ${noun}`,
    ).then(() => setBusy(false));
  };

  return (
    <div className="thread-locked" role="note">
      <Icon name="lock" size={16} />
      <span className="thread-locked-text">
        <strong>This {noun} is locked.</strong>{" "}
        {canUnlock ? "Unlock it to allow replies." : "No one can reply."}
      </span>
      {canUnlock ? (
        <Button variant="secondary" size="sm" icon="lock-open" loading={busy} onClick={unlock}>
          Unlock
        </Button>
      ) : null}
    </div>
  );
}

/** Where the composer goes: the composer, or the locked bar. */
function ThreadFooter({
  roomId,
  threadId,
  noun,
}: {
  readonly roomId: number;
  readonly threadId: number;
  readonly noun: Noun;
}) {
  const status = useStore((state) => state.threads[threadId]?.status ?? "active");

  if (status === "locked") {
    return <LockedComposer threadId={threadId} noun={noun} />;
  }

  return (
    <>
      {status === "closed" ? (
        <p className="thread-composer-note">
          <Icon name="archive" size={12} />
          This {noun} is closed. Replying reopens it.
        </p>
      ) : null}
      <Composer roomId={roomId} threadId={threadId} placeholder="Reply…" />
    </>
  );
}

/** "Delete post": confirms, deletes, and goes back to the room (the board keeps its filters). */
function DeleteDialog({
  roomId,
  threadId,
  noun,
  open,
  onOpenChange,
}: {
  readonly roomId: number;
  readonly threadId: number;
  readonly noun: Noun;
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
}) {
  const navigate = useNavigate();
  const [busy, setBusy] = useState(false);

  const remove = () => {
    setBusy(true);
    void attempt(actions.threads.remove(threadId), `Couldn't delete the ${noun}`).then((done) => {
      setBusy(false);

      if (done) {
        onOpenChange(false);
        toast({ title: `${capitalized(noun)} deleted`, tone: "success" });
        void navigate({ to: "/r/$roomId", params: { roomId }, search: parseBoardSearch });
      }
    });
  };

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      role="alertdialog"
      size="sm"
      title={`Delete ${noun}`}
      description={
        noun === "post"
          ? "Delete this post and its discussion? This can't be undone."
          : "Delete this thread and its replies? This can't be undone."
      }
      footer={
        <>
          <Button variant="secondary" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button variant="danger" loading={busy} onClick={remove}>
            Delete {noun}
          </Button>
        </>
      }
    />
  );
}

/**
 * `/r/$roomId/t/$threadId` in the right pane: subscribes while open, shows the header (name,
 * room, status, follow, actions), the parent and its replies, and the reply composer. Viewing it
 * marks it read; a deleted thread says so.
 */
export function ThreadPane({
  roomId,
  threadId,
}: {
  readonly roomId: number;
  readonly threadId: number;
}) {
  const thread = useStore((state) => state.threads[threadId]);
  const pane = useStore((state) => state.threadPanes[threadId]);

  const unread = useStore(
    (state) => (state.threadMemberships[threadId]?.unreadAt ?? null) !== null,
  );

  const parentId = thread?.parentMessageId ?? null;

  const parent = useStore((state) =>
    parentId === null ? null : (state.messages[parentId] ?? null),
  );

  const tracked = useStore((state) => (state.threads[threadId]?.work ?? null) !== null);
  const [renaming, setRenaming] = useState(false);
  const [archiving, setArchiving] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const navigate = useNavigate();
  const noun = useNoun(roomId);
  const status = pane?.status ?? "loading";
  // A reply's permalink: the pane opens around it and highlights it.
  const focusMessageId = useSearch({ strict: false }).m ?? null;
  const focusRef = useRef(focusMessageId);

  focusRef.current = focusMessageId;

  useEffect(() => {
    void actions.threads.open(threadId, focusRef.current).catch(() => undefined);

    return () => actions.threads.close(threadId);
  }, [threadId]);

  // The thread API only checks membership in the thread's own room, so a link that names a
  // different room would open this thread under that room. Send it to the room it belongs to.
  const elsewhere = thread === undefined ? null : foreignThreadHref(roomId, thread, focusMessageId);

  useEffect(() => {
    if (elsewhere === null) {
      return;
    }

    void navigate({ href: `/app${elsewhere}`, replace: true });
  }, [elsewhere, navigate]);

  // Viewing it reads it: now, and whenever a reply makes it unread while it's on screen.
  useEffect(() => {
    if (status !== "ready" || !unread) {
      return;
    }

    const markIfVisible = () => {
      if (document.visibilityState === "visible") {
        void actions.threads.markRead(threadId).catch(() => undefined);
      }
    };

    markIfVisible();
    document.addEventListener("visibilitychange", markIfVisible);

    return () => document.removeEventListener("visibilitychange", markIfVisible);
  }, [status, unread, threadId]);

  if (elsewhere !== null) {
    return null;
  }

  if (status === "error") {
    const message = pane?.error ?? "This thread couldn't be loaded.";
    const deleted = message === DELETED;

    return (
      <PaneFrame
        title={capitalized(noun)}
        subtitle={<ThreadSubtitle roomId={roomId} threadId={threadId} />}
      >
        <PaneError
          message={message}
          onRetry={
            deleted
              ? undefined
              : () => void actions.threads.reload(threadId, focusMessageId).catch(() => undefined)
          }
        />
      </PaneFrame>
    );
  }

  return (
    <PaneFrame
      title={thread === undefined ? capitalized(noun) : threadTitle(thread)}
      subtitle={<ThreadSubtitle roomId={roomId} threadId={threadId} />}
      tools={
        status === "ready" ? (
          <>
            <FollowButton threadId={threadId} />
            <ThreadMenu
              roomId={roomId}
              threadId={threadId}
              permissions={pane?.permissions ?? null}
              noun={noun}
              onRename={() => setRenaming(true)}
              onAutoArchive={() => setArchiving(true)}
              onDelete={() => setDeleting(true)}
            />
          </>
        ) : null
      }
      toolbar={
        status === "ready" && tracked && noun !== "post" ? (
          <WorkBar threadId={threadId} />
        ) : undefined
      }
      footer={<ThreadFooter roomId={roomId} threadId={threadId} noun={noun} />}
    >
      <ThreadTimeline
        threadId={threadId}
        parent={parent}
        replyCount={thread?.replyCount ?? 0}
        ready={status === "ready"}
        focusMessageId={focusMessageId}
        intro={noun === "post" ? <PostWork threadId={threadId} /> : undefined}
      />
      <RenameDialog threadId={threadId} noun={noun} open={renaming} onOpenChange={setRenaming} />
      <AutoArchiveDialog threadId={threadId} open={archiving} onOpenChange={setArchiving} />
      <DeleteDialog
        roomId={roomId}
        threadId={threadId}
        noun={noun}
        open={deleting}
        onOpenChange={setDeleting}
      />
      <WorkLive threadId={threadId} />
      <HandoffArrival threadId={threadId} />
      <LinksArrival threadId={threadId} />
    </PaneFrame>
  );
}
