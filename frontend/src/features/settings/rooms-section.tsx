import { Link } from "@tanstack/react-router";
import { useCallback, useEffect, useRef, useState } from "react";
import type { AccountSettings } from "../../gen/AccountSettings.ts";
import type { Involvement } from "../../gen/Involvement.ts";
import type { RoomMembershipRow } from "../../gen/RoomMembershipRow.ts";
import { actions } from "../../sync/runtime.ts";
import { settings as settingsActions } from "../../sync/settings.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import { Menu, MenuGroup, MenuRadioItem } from "../../ui/menu.tsx";
import { PaneEmpty, PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import { involvementChoice, levelChoices } from "../sidebar/organize-model.ts";
import { SettingsGroup, SettingsPage, toastFailure } from "./settings-parts.tsx";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly account: AccountSettings };

/**
 * A room's level: `null` for a membership with none stored, which the classic page labels with
 * nothing and which no mention reaches.
 */
type Level = Involvement | null;

/** Each room's level as it now stands, by room id (a change shows before the server answers). */
type Levels = Readonly<Record<number, Level>>;

/** The trigger for a room with no level stored: no choice is selected. */
const UNSET = { label: "Not set", icon: "bell" } as const;

/**
 * Rooms: every room and direct message you're in, in the classic profile's order, each with its
 * notification level. A change goes through the same call as the sidebar's bell menu.
 */
export function RoomsSection() {
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [levels, setLevels] = useState<Levels>({});
  // Per room: the newest change's ticket, and the level the server last accepted. Only the newest
  // change may undo itself, and it goes back to what the server holds, not to what it replaced.
  const latest = useRef(new Map<number, number>());
  const accepted = useRef(new Map<number, Level>());

  const fetchAccount = useCallback(() => {
    settingsActions.account().then(
      (account) => {
        latest.current.clear();
        accepted.current.clear();
        setLevels({});
        setLoad({ status: "ready", account });
      },
      (error: Error) => setLoad({ status: "error", message: error.message }),
    );
  }, []);

  useEffect(fetchAccount, [fetchAccount]);

  const reload = () => {
    setLoad({ status: "loading" });
    fetchAccount();
  };

  const change = (row: RoomMembershipRow, level: Involvement, previous: Level) => {
    const ticket = (latest.current.get(row.roomId) ?? 0) + 1;

    latest.current.set(row.roomId, ticket);

    if (!accepted.current.has(row.roomId)) {
      accepted.current.set(row.roomId, previous);
    }

    setLevels((current) => ({ ...current, [row.roomId]: level }));

    actions.organize.setInvolvement(row.roomId, level).then(
      () => accepted.current.set(row.roomId, level),
      (error: Error) => {
        if (latest.current.get(row.roomId) === ticket) {
          // An accepted level may be null (never set), so ask whether there is one at all.
          const restored = accepted.current.has(row.roomId)
            ? (accepted.current.get(row.roomId) ?? null)
            : previous;

          setLevels((current) => ({ ...current, [row.roomId]: restored }));
        }

        toastFailure(`Couldn't change notifications for ${row.name}`, error);
      },
    );
  };

  const rows = (list: readonly RoomMembershipRow[]) => (
    <ul className="settings-list">
      {list.map((row) => (
        <RoomRow
          key={row.roomId}
          row={row}
          level={row.roomId in levels ? (levels[row.roomId] ?? null) : row.involvement}
          onChange={change}
        />
      ))}
    </ul>
  );

  const empty =
    load.status === "ready" &&
    load.account.sharedRooms.length === 0 &&
    load.account.directRooms.length === 0;

  return (
    <SettingsPage
      title="Rooms"
      description="Every room you're in, and how much each one notifies you."
    >
      {load.status === "loading" ? <PaneListSkeleton rows={4} /> : null}
      {load.status === "error" ? <PaneError message={load.message} onRetry={reload} /> : null}
      {empty ? (
        <PaneEmpty icon="hash" title="No rooms yet" text="Rooms you join show up here." />
      ) : null}
      {load.status === "ready" && load.account.sharedRooms.length > 0 ? (
        <SettingsGroup title="Rooms">{rows(load.account.sharedRooms)}</SettingsGroup>
      ) : null}
      {load.status === "ready" && load.account.directRooms.length > 0 ? (
        <SettingsGroup title="Direct messages">{rows(load.account.directRooms)}</SettingsGroup>
      ) : null}
    </SettingsPage>
  );
}

interface RoomRowProps {
  readonly row: RoomMembershipRow;
  readonly level: Level;
  readonly onChange: (row: RoomMembershipRow, level: Involvement, previous: Level) => void;
}

/** One room: its name (a link into it) and a menu of the levels it offers. */
function RoomRow({ row, level, onChange }: RoomRowProps) {
  const current = level === null ? UNSET : involvementChoice(level);

  return (
    <li className="settings-list-row">
      <span className="settings-list-main">
        <Link to="/r/$roomId" params={{ roomId: row.roomId }} className="settings-room-name">
          {row.name}
        </Link>
      </span>
      <Menu
        placement="bottom-end"
        label={`Notifications for ${row.name}`}
        trigger={(props) => (
          <button
            {...props}
            type="button"
            className="settings-level"
            data-level={level ?? "unset"}
            aria-label={`Notifications for ${row.name}: ${current.label}`}
          >
            <Icon name={current.icon} size={16} />
            <span>{current.label}</span>
            <Icon name="chevron-down" size={14} />
          </button>
        )}
      >
        <MenuGroup label="Notify me about">
          {levelChoices(row.direct).map((choice) => (
            <MenuRadioItem
              key={choice.level}
              icon={choice.icon}
              description={choice.description}
              checked={choice.level === level}
              onSelect={() => {
                if (choice.level !== level) {
                  onChange(row, choice.level, level);
                }
              }}
            >
              {choice.label}
            </MenuRadioItem>
          ))}
        </MenuGroup>
      </Menu>
    </li>
  );
}
