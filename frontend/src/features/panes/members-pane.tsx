import { useNavigate } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import { createDirect } from "../../api/direct-endpoints.ts";
import { members as fetchMembers, setStarred } from "../../api/pane-endpoints.ts";
import type { MemberList } from "../../gen/MemberList.ts";
import { mutations, useStore } from "../../store/store.ts";
import { runAction } from "../../sync/runtime.ts";
import { IconButton } from "../../ui/icon-button.tsx";
import { SkeletonReveal } from "../../ui/skeleton.tsx";
import { toast } from "../../ui/toast-store.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import { canStar, groupMembers, type MemberEntry } from "./members.ts";
import { PaneFrame } from "./pane-frame.tsx";
import { PaneEmpty, PaneError, PaneListSkeleton, PaneSearch } from "./pane-states.tsx";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly list: MemberList };

const PRESENCE_WORD = {
  online: "Active",
  idle: "Away",
  dnd: "Do not disturb",
  offline: "Offline",
} as const;

interface MemberRowProps {
  readonly member: MemberEntry;
  readonly onOpen: (member: MemberEntry) => void;
  readonly onStar: (member: MemberEntry) => void;
}

function MemberRow({ member, onOpen, onStar }: MemberRowProps) {
  const status = member.statusText ?? (member.bot ? null : PRESENCE_WORD[member.presence]);

  return (
    <li className="member-row" data-offline={member.presence === "offline" || undefined}>
      <button
        type="button"
        className="member-row-main"
        disabled={member.viewer}
        aria-label={member.viewer ? `${member.name} (you)` : `Message ${member.name}`}
        onClick={() => onOpen(member)}
      >
        <UserAvatar userId={member.userId} size={32} presence decorative />
        <span className="member-row-text">
          <span className="member-row-name">
            <span className="member-row-label">{member.name}</span>
            {member.viewer ? <span className="member-row-you">(you)</span> : null}
            {member.bot ? <span className="message-agent-tag">Agent</span> : null}
          </span>
          {status === null ? null : <span className="member-row-status">{status}</span>}
        </span>
      </button>
      {canStar(member) ? (
        <IconButton
          icon="star"
          size="sm"
          label={member.starred ? `Unstar ${member.name}` : `Star ${member.name}`}
          className="member-star"
          data-starred={member.starred || undefined}
          aria-pressed={member.starred}
          tooltipPlacement="left"
          onClick={() => onStar(member)}
        />
      ) : null}
    </li>
  );
}

/**
 * The Members pane: everyone active in the room, searchable, in Starred, Online and Offline
 * sections with live presence dots and status lines. Star people to keep them on top; choose
 * someone to open your direct message with them.
 */
export function MembersPane({ roomId }: { readonly roomId: number }) {
  const navigate = useNavigate();
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [query, setQuery] = useState("");
  const [stars, setStars] = useState<Readonly<Record<number, boolean>>>({});
  const users = useStore((state) => state.users);
  const presence = useStore((state) => state.presence);
  const viewerId = useStore((state) => state.me?.user.id ?? state.boot?.user.id ?? null);

  const reload = useCallback(() => {
    setLoad({ status: "loading" });
    runAction(fetchMembers(roomId)).then(
      (list) => {
        mutations.mergeUsers(list.users);
        setStars({});
        setLoad({ status: "ready", list });
      },
      (error: Error) => setLoad({ status: "error", message: error.message }),
    );
  }, [roomId]);

  useEffect(reload, [reload]);

  const star = (member: MemberEntry) => {
    const next = !member.starred;

    setStars((held) => ({ ...held, [member.userId]: next }));
    runAction(setStarred(member.userId, next)).then(
      (reply) => setStars((held) => ({ ...held, [reply.userId]: reply.starred })),
      (error: Error) => {
        setStars((held) => ({ ...held, [member.userId]: member.starred }));
        toast({
          title: `Couldn't ${next ? "star" : "unstar"} ${member.name}`,
          description: error.message,
          tone: "danger",
        });
      },
    );
  };

  const open = (member: MemberEntry) => {
    runAction(createDirect([member.userId])).then(
      (row) => void navigate({ to: "/r/$roomId", params: { roomId: row.room.id } }),
      (error: Error) =>
        toast({
          title: `Couldn't message ${member.name}`,
          description: error.message,
          tone: "danger",
        }),
    );
  };

  const sections =
    load.status === "ready"
      ? groupMembers({ members: load.list.members, users, presence, stars, query, viewerId })
      : [];

  const total = load.status === "ready" ? load.list.members.length : null;

  return (
    <PaneFrame
      title="Members"
      subtitle={total === null ? undefined : `${total} ${total === 1 ? "person" : "people"}`}
      toolbar={<PaneSearch value={query} onValueChange={setQuery} label="Find a member" />}
    >
      {load.status === "error" ? (
        <PaneError message="The member list couldn't be loaded." onRetry={reload} />
      ) : (
        <SkeletonReveal
          loading={load.status === "loading"}
          skeleton={<PaneListSkeleton rows={8} />}
        >
          {load.status === "ready" && sections.length === 0 ? (
            <PaneEmpty
              icon="users"
              title={query.trim() === "" ? "No one here" : "No matches"}
              text={
                query.trim() === ""
                  ? "This room has no active members."
                  : `No member is called "${query.trim()}".`
              }
            />
          ) : (
            <div className="member-sections">
              {sections.map((section) => (
                <section
                  key={section.key}
                  className="member-section"
                  aria-labelledby={`members-${section.key}`}
                >
                  <h3 id={`members-${section.key}`} className="pane-section-title">
                    {section.label}
                    <span className="pane-section-count tabular">{section.members.length}</span>
                  </h3>
                  <ul className="member-list">
                    {section.members.map((member) => (
                      <MemberRow key={member.userId} member={member} onOpen={open} onStar={star} />
                    ))}
                  </ul>
                </section>
              ))}
            </div>
          )}
        </SkeletonReveal>
      )}
    </PaneFrame>
  );
}
