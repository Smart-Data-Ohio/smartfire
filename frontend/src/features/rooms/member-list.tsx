import { type KeyboardEvent, useId, useState } from "react";
import type { RoomFormStageRole } from "../../gen/RoomFormStageRole.ts";
import { useStore } from "../../store/store.ts";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { pickerOptions } from "../directs/picker.ts";
import { UNKNOWN_NAME, useUser } from "../people/people.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import "../directs/directs.css";

const ROLE_LABEL = { host: "Host", speaker: "Speaker", listener: "Listener" } as const;

/** How many matches the add field lists under itself. */
const MAX_MATCHES = 6;

interface MemberListProps {
  /** Everyone who could be in the room (active people, in the server's order). */
  readonly candidateIds: readonly number[];
  /** Who is in it in the draft. */
  readonly memberIds: readonly number[];
  /** Who was in it when the form loaded, so new people are marked. */
  readonly savedIds: readonly number[];
  readonly onChange: (memberIds: readonly number[]) => void;
  readonly viewerId: number;
  readonly stageRoles: readonly RoomFormStageRole[];
  readonly agentIds: ReadonlySet<number>;
  readonly readOnly: boolean;
}

function MemberRow({
  userId,
  isViewer,
  isNew,
  role,
  agent,
  onRemove,
}: {
  readonly userId: number;
  readonly isViewer: boolean;
  readonly isNew: boolean;
  readonly role: RoomFormStageRole["role"] | null;
  readonly agent: boolean;
  readonly onRemove: (() => void) | null;
}) {
  const name = useUser(userId)?.name ?? UNKNOWN_NAME;

  return (
    <li className="member-row enter-rise">
      <UserAvatar userId={userId} size={28} presence decorative />
      <span className="member-row-name">
        {name}
        {isViewer ? <span className="member-row-you"> (you)</span> : null}
      </span>
      {isNew ? (
        <span className="member-tag" data-tone="accent">
          New
        </span>
      ) : null}
      {agent ? <span className="member-tag">Agent</span> : null}
      {role === null ? null : (
        <span className="member-tag" data-tone={role === "host" ? "accent" : undefined}>
          {role === "host" ? <Icon name="crown" size={11} /> : null}
          {ROLE_LABEL[role]}
        </span>
      )}
      {onRemove === null ? null : (
        <IconButton
          icon="user-x"
          label={isViewer ? "Leave (remove yourself)" : `Remove ${name}`}
          size="sm"
          className="member-row-remove"
          onClick={onRemove}
        />
      )}
    </li>
  );
}

/**
 * A room's people as a list (Discord's channel permissions member list): an "Add people" search
 * on top whose matches drop in below it, then everyone in the draft with their stage role and a
 * remove button. Changes stay in the draft until the dialog saves.
 */
export function MemberList({
  candidateIds,
  memberIds,
  savedIds,
  onChange,
  viewerId,
  stageRoles,
  agentIds,
  readOnly,
}: MemberListProps) {
  const id = useId();
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const users = useStore((state) => state.users);
  const members = new Set(memberIds);
  const saved = new Set(savedIds);
  const roles = new Map(stageRoles.map((entry) => [entry.userId, entry.role]));

  const candidates = candidateIds.map((userId) => ({
    userId,
    agent: agentIds.has(userId),
    starred: false,
  }));

  const matches =
    query.trim() === ""
      ? []
      : pickerOptions(candidates, users, query, members).slice(0, MAX_MATCHES);

  const active = Math.min(activeIndex, Math.max(matches.length - 1, 0));
  const activeMatch = matches[active];

  const add = (userId: number) => {
    onChange([...memberIds, userId]);
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

  // The viewer first, then the people already in, then anyone just added (in the order added).
  const ordered = [
    ...memberIds.filter((userId) => userId === viewerId),
    ...memberIds.filter((userId) => userId !== viewerId && saved.has(userId)),
    ...memberIds.filter((userId) => userId !== viewerId && !saved.has(userId)),
  ];

  return (
    <div className="member-list">
      {readOnly ? null : (
        <div className="member-add">
          <label className="member-add-field">
            <Icon name="user-plus" size={16} className="member-add-icon" />
            <input
              className="member-add-input"
              type="text"
              role="combobox"
              aria-label="Add people"
              aria-expanded={matches.length > 0}
              aria-controls={`${id}-matches`}
              aria-autocomplete="list"
              aria-activedescendant={
                activeMatch === undefined ? undefined : `${id}-${activeMatch.userId}`
              }
              placeholder="Add people by name"
              autoComplete="off"
              spellCheck={false}
              value={query}
              onChange={(event) => {
                setQuery(event.target.value);
                setActiveIndex(0);
              }}
              onKeyDown={onKeyDown}
            />
          </label>
          {query.trim() === "" ? null : (
            <div
              id={`${id}-matches`}
              role="listbox"
              aria-label="People"
              className="member-matches enter-fade"
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
                  <Icon name="plus" size={14} className="member-match-add" />
                </div>
              ))}
              {matches.length === 0 ? (
                <p className="picker-empty">Nobody else matches “{query.trim()}”.</p>
              ) : null}
            </div>
          )}
        </div>
      )}
      <p className="room-form-legend" id={`${id}-count`}>
        {memberIds.length === 1 ? "1 member" : `${memberIds.length} members`}
      </p>
      <ul className="member-rows" aria-labelledby={`${id}-count`}>
        {ordered.map((userId) => (
          <MemberRow
            key={userId}
            userId={userId}
            isViewer={userId === viewerId}
            isNew={!saved.has(userId)}
            role={roles.get(userId) ?? null}
            agent={agentIds.has(userId)}
            onRemove={
              readOnly ? null : () => onChange(memberIds.filter((member) => member !== userId))
            }
          />
        ))}
      </ul>
    </div>
  );
}
