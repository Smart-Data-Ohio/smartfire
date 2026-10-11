import { Link } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import type { BlockedPerson } from "../../gen/BlockedPerson.ts";
import { isIgnored } from "../../store/ignores.ts";
import { useStore } from "../../store/store.ts";
import { ignores } from "../../sync/admin.ts";
import { Button } from "../../ui/button.tsx";
import { toast } from "../../ui/toast-store.ts";
import { PaneEmpty, PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import { UNKNOWN_NAME } from "../people/people.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import { timeAgo } from "../threads/thread-format.ts";
import { useNow } from "../threads/use-now.ts";
import { SettingsGroup, SettingsPage, toastFailure, useBusy } from "./settings-parts.tsx";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly people: readonly BlockedPerson[] };

/** One person you ignore: who, since when, and the way to stop. */
function IgnoredRow({
  person,
  now,
  busy,
  onStop,
}: {
  readonly person: BlockedPerson;
  readonly now: number;
  readonly busy: boolean;
  readonly onStop: () => void;
}) {
  const user = useStore((state) => state.users[person.user.id]) ?? person.user;
  const name = user.name === "" ? UNKNOWN_NAME : user.name;

  return (
    <li className="settings-list-row">
      <UserAvatar userId={user.id} size={32} decorative />
      <span className="settings-list-main">
        <Link
          to="/people/$userId"
          params={{ userId: user.id }}
          className="settings-room-name"
          preload={false}
        >
          {name}
        </Link>
        <span className="text-faint">Ignored {timeAgo(person.createdAt, now)}</span>
      </span>
      <Button
        variant="secondary"
        size="sm"
        loading={busy}
        aria-label={`Stop ignoring ${name}`}
        onClick={onStop}
      >
        Stop ignoring
      </Button>
    </li>
  );
}

/**
 * Ignored people: everyone whose messages you've collapsed and who can't start new DMs with you,
 * oldest first, each with a way to stop. Private to you; nobody on the list is told. A change made
 * elsewhere (their profile, another session) drops them from the list as it lands.
 */
export function BlockedSection() {
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const ignoreSet = useStore((state) => state.ignores);
  const { busy, track } = useBusy();
  const now = useNow();

  const fetchList = useCallback(() => {
    ignores.list().then(
      (list) => setLoad({ status: "ready", people: list.people }),
      (error: Error) => setLoad({ status: "error", message: error.message }),
    );
  }, []);

  useEffect(fetchList, [fetchList]);

  const reload = () => {
    setLoad({ status: "loading" });
    fetchList();
  };

  const stop = (person: BlockedPerson) => {
    const key = `person-${person.user.id}`;

    void track(key, ignores.set(person.user.id, false)).then(
      () => toast({ title: `You stopped ignoring ${person.user.name}`, tone: "success" }),
      (error: Error) => toastFailure(`Couldn't stop ignoring ${person.user.name}`, error),
    );
  };

  const people =
    load.status === "ready"
      ? load.people.filter((person) => isIgnored(ignoreSet, person.user.id, person.ignoredByMe))
      : [];

  return (
    <SettingsPage
      title="Ignored people"
      description="Their messages stay collapsed for you, and they can't start new DMs with you. Only you see this list."
    >
      {load.status === "loading" ? <PaneListSkeleton rows={2} /> : null}
      {load.status === "error" ? <PaneError message={load.message} onRetry={reload} /> : null}
      {load.status === "ready" && people.length === 0 ? (
        <PaneEmpty
          icon="eye-off"
          title="You're not ignoring anyone"
          text="Ignore someone from their profile to collapse their messages."
        />
      ) : null}
      {people.length > 0 ? (
        <SettingsGroup title="Ignored">
          <ul className="settings-list">
            {people.map((person) => (
              <IgnoredRow
                key={person.user.id}
                person={person}
                now={now}
                busy={busy(`person-${person.user.id}`)}
                onStop={() => stop(person)}
              />
            ))}
          </ul>
        </SettingsGroup>
      ) : null}
    </SettingsPage>
  );
}
