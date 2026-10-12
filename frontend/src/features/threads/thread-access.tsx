/**
 * Private threads: the lock wherever one is shown, and its people. Anyone who can see a private
 * thread may add people from its room; the creator and admins may remove them; anyone it was
 * shared with may leave. Admins see every private thread in their rooms without being added.
 * Access is separate from following: being added doesn't make anyone follow, and unfollowing
 * doesn't take access away.
 */
import { type KeyboardEvent, useCallback, useEffect, useId, useState } from "react";
import type { ThreadAccessMember } from "../../gen/ThreadAccessMember.ts";
import { useStore } from "../../store/store.ts";
import { panes } from "../../sync/panes.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { toast } from "../../ui/toast-store.ts";
import { pickerOptions } from "../directs/picker.ts";
import { PaneListSkeleton } from "../panes/pane-states.tsx";
import { UNKNOWN_NAME, useUser } from "../people/people.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import { threadTitle } from "./thread-format.ts";
import "../directs/directs.css";
import "./threads.css";

/** How many matches the add field lists under itself. */
const MAX_MATCHES = 6;

/** What the viewer may do in a private thread's member list, as the server allows it. */
export interface ThreadAccessRights {
  /** The creator or an admin: may remove anyone. */
  readonly canRemove: boolean;
  /** Shared with the viewer (not an admin, who keeps seeing it): may leave. */
  readonly canLeave: boolean;
}

export function accessRights(
  viewerId: number | null,
  administrator: boolean,
  creatorId: number | null,
  members: readonly ThreadAccessMember[] | null,
): ThreadAccessRights {
  const shared = members?.some((member) => member.userId === viewerId) === true;

  return {
    canRemove: administrator || (viewerId !== null && viewerId === creatorId),
    canLeave: shared && !administrator,
  };
}

/** The private thread's member list as the pane holds it. */
export interface ThreadAccess {
  /** `null` until the first answer. */
  readonly members: readonly ThreadAccessMember[] | null;
  readonly error: string | null;
  readonly reload: () => void;
  readonly replace: (members: readonly ThreadAccessMember[]) => void;
  readonly drop: (userId: number) => void;
}

interface Held {
  readonly threadId: number;
  readonly members: readonly ThreadAccessMember[] | null;
  readonly error: string | null;
}

/** Loads a private thread's members while `enabled`, again on `reload`. */
export function useThreadAccess(threadId: number, enabled: boolean): ThreadAccess {
  const [held, setHeld] = useState<Held>({ threadId, members: null, error: null });
  const [ticket, setTicket] = useState(0);
  // Another thread's list never shows under this one while its own loads.
  const current = held.threadId === threadId ? held : { threadId, members: null, error: null };

  // biome-ignore lint/correctness/useExhaustiveDependencies: ticket is the reload trigger
  useEffect(() => {
    if (!enabled) {
      return;
    }

    let live = true;

    actions.threads.accessMembers(threadId).then(
      (list) => {
        if (live) setHeld({ threadId, members: list.members, error: null });
      },
      (error: Error) => {
        if (live) {
          setHeld((last) => ({
            threadId,
            members: last.threadId === threadId ? last.members : null,
            error: error.message,
          }));
        }
      },
    );

    return () => {
      live = false;
    };
  }, [threadId, enabled, ticket]);

  const reload = useCallback(() => setTicket((count) => count + 1), []);

  const replace = useCallback(
    (members: readonly ThreadAccessMember[]) => setHeld({ threadId, members, error: null }),
    [threadId],
  );

  const drop = useCallback(
    (userId: number) =>
      setHeld((last) => ({
        ...last,
        members: last.members?.filter((member) => member.userId !== userId) ?? null,
      })),
    [],
  );

  return { members: current.members, error: current.error, reload, replace, drop };
}

/** "1 person", "3 people". */
function peopleCount(count: number): string {
  return count === 1 ? "1 person" : `${count} people`;
}

/** The header's people button: the count, opening the member list. */
export function ThreadMembersButton({
  count,
  onOpen,
}: {
  readonly count: number | null;
  readonly onOpen: () => void;
}) {
  const label = count === null ? "Members" : `Members: ${peopleCount(count)}`;

  return (
    <Button
      variant="ghost"
      size="sm"
      icon="users"
      className="thread-members-button"
      aria-label={label}
      aria-haspopup="dialog"
      onClick={onOpen}
    >
      {count === null ? null : <span className="tabular">{count}</span>}
    </Button>
  );
}

function MemberRow({
  userId,
  isViewer,
  isCreator,
  busy,
  onRemove,
  onLeave,
}: {
  readonly userId: number;
  readonly isViewer: boolean;
  readonly isCreator: boolean;
  readonly busy: boolean;
  readonly onRemove: (() => void) | null;
  readonly onLeave: (() => void) | null;
}) {
  const name = useUser(userId)?.name ?? UNKNOWN_NAME;

  return (
    <li className="thread-access-row enter-rise">
      <UserAvatar userId={userId} size={28} presence decorative />
      <span className="thread-access-name">
        {name}
        {isViewer ? <span className="thread-access-you"> (you)</span> : null}
      </span>
      {isCreator ? <span className="thread-access-tag">Creator</span> : null}
      {onLeave === null ? null : (
        <Button variant="ghost" size="sm" className="thread-access-leave" onClick={onLeave}>
          Leave
        </Button>
      )}
      {onRemove === null ? null : (
        <IconButton
          icon="user-x"
          label={`Remove ${name}`}
          size="sm"
          className="thread-access-remove"
          loading={busy}
          onClick={onRemove}
        />
      )}
    </li>
  );
}

/**
 * "Add people": a search over the room's active members who can't see the thread yet. Picking
 * one shares the thread with them at once.
 */
function AddPeople({
  roomId,
  excluded,
  busy,
  onAdd,
}: {
  readonly roomId: number;
  readonly excluded: ReadonlySet<number>;
  readonly busy: number | null;
  readonly onAdd: (userId: number) => void;
}) {
  const id = useId();
  const users = useStore((state) => state.users);
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const [roomMembers, setRoomMembers] = useState<readonly number[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;

    panes.members(roomId).then(
      (list) => {
        if (live) setRoomMembers(list.members.map((member) => member.userId));
      },
      (failure: Error) => {
        if (live) setError(failure.message);
      },
    );

    return () => {
      live = false;
    };
  }, [roomId]);

  const candidates = (roomMembers ?? []).map((userId) => ({
    userId,
    agent: users[userId]?.agent != null,
    starred: false,
  }));

  const matches =
    query.trim() === ""
      ? []
      : pickerOptions(candidates, users, query, excluded).slice(0, MAX_MATCHES);

  const active = Math.min(activeIndex, Math.max(matches.length - 1, 0));
  const activeMatch = matches[active];

  const add = (userId: number) => {
    if (busy !== null) return;

    onAdd(userId);
    setQuery("");
    setActiveIndex(0);
  };

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.nativeEvent.isComposing) return;

    if ((event.key === "ArrowDown" || event.key === "ArrowUp") && matches.length > 0) {
      event.preventDefault();
      setActiveIndex(
        (active + (event.key === "ArrowDown" ? 1 : -1) + matches.length) % matches.length,
      );
    } else if (event.key === "Enter" && activeMatch !== undefined) {
      event.preventDefault();
      add(activeMatch.userId);
    } else if (event.key === "Escape" && query !== "") {
      // Clears the search first; a second Escape closes the dialog.
      event.preventDefault();
      event.stopPropagation();
      setQuery("");
    }
  };

  return (
    <div className="thread-access-add">
      <label className="thread-access-field">
        <Icon name="user-plus" size={16} className="thread-access-field-icon" />
        <input
          className="thread-access-input"
          type="text"
          role="combobox"
          aria-label="Add people"
          aria-expanded={matches.length > 0}
          aria-controls={`${id}-matches`}
          aria-autocomplete="list"
          aria-activedescendant={
            activeMatch === undefined ? undefined : `${id}-${activeMatch.userId}`
          }
          placeholder="Add people from this room"
          autoComplete="off"
          spellCheck={false}
          value={query}
          data-autofocus
          onChange={(event) => {
            setQuery(event.target.value);
            setActiveIndex(0);
          }}
          onKeyDown={onKeyDown}
        />
      </label>
      {error !== null && roomMembers === null ? (
        <p className="thread-access-note" data-tone="danger">
          Couldn't load the room's people: {error}
        </p>
      ) : null}
      {query.trim() === "" ? null : (
        <div
          id={`${id}-matches`}
          role="listbox"
          aria-label="People in this room"
          className="thread-access-matches enter-fade"
        >
          {matches.map((match) => (
            // The input owns the keyboard (aria-activedescendant); options take clicks.
            // biome-ignore lint/a11y/useKeyWithClickEvents: the combobox input handles the keys.
            <div
              key={match.userId}
              id={`${id}-${match.userId}`}
              role="option"
              tabIndex={-1}
              aria-selected={false}
              aria-disabled={busy !== null || undefined}
              className="picker-option"
              data-active={match === activeMatch || undefined}
              onPointerMove={() => setActiveIndex(matches.indexOf(match))}
              onClick={() => add(match.userId)}
            >
              <UserAvatar userId={match.userId} size={24} presence decorative />
              <span className="picker-option-text">
                <span className="picker-option-name">{match.name}</span>
              </span>
              {match.agent ? <span className="picker-tag">Agent</span> : null}
              <Icon name="plus" size={14} className="thread-access-match-add" />
            </div>
          ))}
          {matches.length === 0 ? (
            <p className="picker-empty">
              {roomMembers === null
                ? "Loading the room's people…"
                : `Nobody else in this room matches “${query.trim()}”.`}
            </p>
          ) : null}
        </div>
      )}
    </div>
  );
}

interface ThreadMembersDialogProps {
  readonly roomId: number;
  readonly threadId: number;
  readonly access: ThreadAccess;
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
  /** The viewer's own Leave: the pane confirms it. */
  readonly onLeave: () => void;
}

/**
 * Who a private thread is shared with: add people from the room, remove them (the creator and
 * admins), or leave. Changes apply at once; a refusal says why under the search.
 */
export function ThreadMembersDialog({
  roomId,
  threadId,
  access,
  open,
  onOpenChange,
  onLeave,
}: ThreadMembersDialogProps) {
  const thread = useStore((state) => state.threads[threadId]);
  const viewerId = useStore((state) => state.me?.user.id ?? state.boot?.user.id ?? null);
  const administrator = useStore((state) => state.me?.user.role === "administrator");
  const users = useStore((state) => state.users);
  const [busy, setBusy] = useState<number | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [wasOpen, setWasOpen] = useState(open);
  const { members, error, reload, replace, drop } = access;
  const creatorId = thread?.creatorId ?? null;
  const rights = accessRights(viewerId, administrator, creatorId, members);

  if (open !== wasOpen) {
    setWasOpen(open);

    if (open) {
      setBusy(null);
      setProblem(null);
    }
  }

  // Each opening asks afresh: mentions and other people add members meanwhile.
  useEffect(() => {
    if (open) reload();
  }, [open, reload]);

  const nameOf = (userId: number) => users[userId]?.name ?? UNKNOWN_NAME;

  const add = (userId: number) => {
    setBusy(userId);
    setProblem(null);
    actions.threads.invite(threadId, userId).then(
      (list) => {
        setBusy(null);
        replace(list.members);
        toast({ title: `Added ${nameOf(userId)} to the thread`, tone: "success" });
      },
      (failure: Error) => {
        setBusy(null);
        setProblem(failure.message);
      },
    );
  };

  const remove = (userId: number) => {
    setBusy(userId);
    setProblem(null);
    actions.threads.removeMember(threadId, userId).then(
      () => {
        setBusy(null);
        drop(userId);
        toast({ title: `Removed ${nameOf(userId)} from the thread`, tone: "success" });
      },
      (failure: Error) => {
        setBusy(null);
        setProblem(failure.message);
      },
    );
  };

  // The viewer first, then the creator, then everyone else as they were added.
  const ordered =
    members === null
      ? []
      : [
          ...members.filter((member) => member.userId === viewerId),
          ...members.filter((member) => member.userId !== viewerId && member.userId === creatorId),
          ...members.filter((member) => member.userId !== viewerId && member.userId !== creatorId),
        ];

  const excluded = new Set((members ?? []).map((member) => member.userId));
  const name = thread === undefined ? "this thread" : `“${threadTitle(thread)}”`;
  const shared = members?.some((member) => member.userId === viewerId) === true;

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      title="Private thread members"
      description={`Only the people here and admins can see ${name}. Mentioning someone from the room in a reply adds them too.`}
      footer={
        <Button variant="secondary" onClick={() => onOpenChange(false)}>
          Done
        </Button>
      }
    >
      <div className="thread-access">
        <AddPeople roomId={roomId} excluded={excluded} busy={busy} onAdd={add} />
        {problem === null ? null : (
          <p className="thread-access-note" data-tone="danger" role="alert">
            {problem}
          </p>
        )}
        {administrator && members !== null && !shared ? (
          <p className="thread-access-note">
            <Icon name="shield" size={12} />
            You can see it because you're an admin.
          </p>
        ) : null}
        {members === null ? (
          error === null ? (
            <PaneListSkeleton rows={3} square={28} />
          ) : (
            <div className="thread-access-error" role="alert">
              <p>Couldn't load the members: {error}</p>
              <Button variant="secondary" size="sm" icon="rotate-ccw" onClick={reload}>
                Try again
              </Button>
            </div>
          )
        ) : members.length === 0 ? (
          <p className="thread-access-empty">
            Nobody has been added yet. Add people from the room above.
          </p>
        ) : (
          <ul
            className="thread-access-rows"
            aria-label={members.length === 1 ? "1 member" : `${members.length} members`}
          >
            {ordered.map((member) => {
              const isViewer = member.userId === viewerId;

              return (
                <MemberRow
                  key={member.userId}
                  userId={member.userId}
                  isViewer={isViewer}
                  isCreator={member.userId === creatorId}
                  busy={busy === member.userId}
                  onLeave={isViewer && rights.canLeave ? onLeave : null}
                  onRemove={
                    !isViewer && rights.canRemove && busy === null
                      ? () => remove(member.userId)
                      : null
                  }
                />
              );
            })}
          </ul>
        )}
      </div>
    </Dialog>
  );
}

/** Confirms leaving a private thread: the viewer loses it until someone adds them back. */
export function LeaveThreadDialog({
  threadId,
  open,
  onOpenChange,
  onLeft,
}: {
  readonly threadId: number;
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
  readonly onLeft: () => void;
}) {
  const viewerId = useStore((state) => state.me?.user.id ?? state.boot?.user.id ?? null);
  const [busy, setBusy] = useState(false);

  const leave = () => {
    if (viewerId === null) return;

    setBusy(true);
    actions.threads.removeMember(threadId, viewerId).then(
      () => {
        setBusy(false);
        onOpenChange(false);
        toast({ title: "You left the thread", tone: "success" });
        onLeft();
      },
      (failure: Error) => {
        setBusy(false);
        toast({ title: "Couldn't leave the thread", description: failure.message, tone: "danger" });
      },
    );
  };

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      role="alertdialog"
      size="sm"
      title="Leave private thread"
      description="You won't see this thread or its replies any more. Someone in it can add you back."
      footer={
        <>
          <Button variant="secondary" data-autofocus onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button variant="danger" loading={busy} onClick={leave}>
            Leave thread
          </Button>
        </>
      }
    />
  );
}
