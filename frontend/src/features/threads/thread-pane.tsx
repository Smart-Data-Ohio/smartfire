import { useEffect, useState } from "react";
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
import { Composer } from "../composer/composer.tsx";
import { PaneFrame, RoomName } from "../panes/pane-frame.tsx";
import { PaneError } from "../panes/pane-states.tsx";
import { THREAD_STATUS_LABEL, threadTitle } from "./thread-format.ts";
import { ThreadTimeline } from "./thread-timeline.tsx";

const DELETED = "This thread was deleted.";

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
  readonly onRename: () => void;
}

/** Copy link, then what the viewer's permissions allow: rename, close or reopen, lock or unlock. */
function ThreadMenu({ roomId, threadId, permissions, onRename }: ThreadMenuProps) {
  const status = useStore((state) => state.threads[threadId]?.status ?? "active");

  const update = (body: Parameters<typeof actions.threads.update>[1], failure: string) =>
    void attempt(actions.threads.update(threadId, body), failure);

  const canClose = permissions?.canClose === true && status === "active";
  const canReopen = permissions?.canReopen === true && status === "closed";
  const canLock = permissions?.canLock === true && status !== "locked";
  const canUnlock = permissions?.canUnlock === true && status === "locked";

  const moderates =
    permissions?.canRename === true || canClose || canReopen || canLock || canUnlock;

  return (
    <Menu
      placement="bottom-end"
      label="Thread actions"
      trigger={(props) => (
        <IconButton {...props} icon="more" label="Thread actions" tooltipPlacement="bottom" />
      )}
    >
      <MenuItem icon="link" onSelect={() => copyLink(roomId, threadId)}>
        Copy link
      </MenuItem>
      {moderates ? <MenuSeparator /> : null}
      {permissions?.canRename === true ? (
        <MenuItem icon="pencil" onSelect={onRename}>
          Rename thread…
        </MenuItem>
      ) : null}
      {canClose ? (
        <MenuItem
          icon="archive"
          onSelect={() => update({ name: null, status: "closed" }, "Couldn't close the thread")}
        >
          Close thread
        </MenuItem>
      ) : null}
      {canReopen ? (
        <MenuItem
          icon="rotate-ccw"
          onSelect={() => update({ name: null, status: "active" }, "Couldn't reopen the thread")}
        >
          Reopen thread
        </MenuItem>
      ) : null}
      {canLock ? (
        <MenuItem
          icon="lock"
          onSelect={() => update({ name: null, status: "locked" }, "Couldn't lock the thread")}
        >
          Lock thread
        </MenuItem>
      ) : null}
      {canUnlock ? (
        <MenuItem
          icon="lock-open"
          onSelect={() => update({ name: null, status: "active" }, "Couldn't unlock the thread")}
        >
          Unlock thread
        </MenuItem>
      ) : null}
    </Menu>
  );
}

/** Renames the thread: 1 to 100 characters. */
function RenameDialog({
  threadId,
  open,
  onOpenChange,
}: {
  readonly threadId: number;
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
}) {
  const current = useStore((state) => state.threads[threadId]?.name ?? "");
  const [name, setName] = useState(current);
  const [error, setError] = useState<string | undefined>(undefined);
  const [attempts, setAttempts] = useState(0);
  const [saving, setSaving] = useState(false);
  const [wasOpen, setWasOpen] = useState(open);

  if (open !== wasOpen) {
    setWasOpen(open);

    if (open) {
      setName(current);
      setError(undefined);
    }
  }

  const save = () => {
    const trimmed = name.trim();

    if (trimmed === "" || trimmed.length > 100) {
      setError(trimmed === "" ? "Give the thread a name." : "Keep it to 100 characters.");
      setAttempts((count) => count + 1);

      return;
    }

    setSaving(true);
    actions.threads.update(threadId, { name: trimmed, status: null }).then(
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
      title="Rename thread"
      size="sm"
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

/**
 * A locked thread refuses every reply, moderators' too, so the composer gives way to a disabled
 * bar that says why; whoever may unlock it gets the button right there.
 */
function LockedComposer({ threadId }: { readonly threadId: number }) {
  const canUnlock = useStore(
    (state) => state.threadPanes[threadId]?.permissions?.canUnlock === true,
  );

  const [busy, setBusy] = useState(false);

  const unlock = () => {
    setBusy(true);
    void attempt(
      actions.threads.update(threadId, { name: null, status: "active" }),
      "Couldn't unlock the thread",
    ).then(() => setBusy(false));
  };

  return (
    <div className="thread-locked" role="note">
      <Icon name="lock" size={16} />
      <span className="thread-locked-text">
        <strong>This thread is locked.</strong>{" "}
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
}: {
  readonly roomId: number;
  readonly threadId: number;
}) {
  const status = useStore((state) => state.threads[threadId]?.status ?? "active");

  if (status === "locked") {
    return <LockedComposer threadId={threadId} />;
  }

  return (
    <>
      {status === "closed" ? (
        <p className="thread-composer-note">
          <Icon name="archive" size={12} />
          This thread is closed. Replying reopens it.
        </p>
      ) : null}
      <Composer roomId={roomId} threadId={threadId} placeholder="Reply…" />
    </>
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

  const [renaming, setRenaming] = useState(false);
  const status = pane?.status ?? "loading";

  useEffect(() => {
    void actions.threads.open(threadId).catch(() => undefined);

    return () => actions.threads.close(threadId);
  }, [threadId]);

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

  if (status === "error") {
    const message = pane?.error ?? "This thread couldn't be loaded.";
    const deleted = message === DELETED;

    return (
      <PaneFrame title="Thread" subtitle={<ThreadSubtitle roomId={roomId} threadId={threadId} />}>
        <PaneError
          message={message}
          onRetry={
            deleted ? undefined : () => void actions.threads.open(threadId).catch(() => undefined)
          }
        />
      </PaneFrame>
    );
  }

  return (
    <PaneFrame
      title={thread === undefined ? "Thread" : threadTitle(thread)}
      subtitle={<ThreadSubtitle roomId={roomId} threadId={threadId} />}
      tools={
        status === "ready" ? (
          <>
            <FollowButton threadId={threadId} />
            <ThreadMenu
              roomId={roomId}
              threadId={threadId}
              permissions={pane?.permissions ?? null}
              onRename={() => setRenaming(true)}
            />
          </>
        ) : null
      }
      footer={<ThreadFooter roomId={roomId} threadId={threadId} />}
    >
      <ThreadTimeline
        threadId={threadId}
        parent={parent}
        replyCount={thread?.replyCount ?? 0}
        ready={status === "ready"}
      />
      <RenameDialog threadId={threadId} open={renaming} onOpenChange={setRenaming} />
    </PaneFrame>
  );
}
