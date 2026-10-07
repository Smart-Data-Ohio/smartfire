import { Link } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
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

/** Each room's level as it now stands, by room id (a change shows before the server answers). */
type Levels = Readonly<Record<number, Involvement>>;

/**
 * Rooms: every room and direct message you're in, in the classic profile's order, each with its
 * notification level. A change goes through the same call as the sidebar's bell menu.
 */
export function RoomsSection() {
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [levels, setLevels] = useState<Levels>({});

  const fetchAccount = useCallback(() => {
    settingsActions.account().then(
      (account) => {
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

  const change = (row: RoomMembershipRow, level: Involvement, previous: Involvement) => {
    setLevels((current) => ({ ...current, [row.roomId]: level }));

    actions.organize.setInvolvement(row.roomId, level).catch((error: Error) => {
      setLevels((current) => ({ ...current, [row.roomId]: previous }));
      toastFailure(`Couldn't change notifications for ${row.name}`, error);
    });
  };

  const rows = (list: readonly RoomMembershipRow[]) => (
    <ul className="settings-list">
      {list.map((row) => (
        <RoomRow
          key={row.roomId}
          row={row}
          level={levels[row.roomId] ?? row.involvement}
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
  readonly level: Involvement;
  readonly onChange: (row: RoomMembershipRow, level: Involvement, previous: Involvement) => void;
}

/** One room: its name (a link into it) and a menu of the levels it offers. */
function RoomRow({ row, level, onChange }: RoomRowProps) {
  const current = involvementChoice(level);

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
            data-level={level}
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
