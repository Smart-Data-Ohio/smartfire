import { Link } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import type { PeoplePage } from "../../gen/PeoplePage.ts";
import type { Person } from "../../gen/Person.ts";
import { admin } from "../../sync/admin.ts";
import { Avatar } from "../../ui/avatar.tsx";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { toast } from "../../ui/toast-store.ts";
import { PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import { SettingsGroup, SettingsPage, useBusy } from "../settings/settings-parts.tsx";
import {
  googleLinkConfirmation,
  googleLinkTitle,
  googleUnlinkConfirmation,
  googleUnlinkTitle,
  groupPeople,
  REMOVE_CONFIRMATION,
  twoFactorResetConfirmation,
} from "./admin-format.ts";
import { adminFailure, useAdmin } from "./admin-parts.tsx";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly page: PeoplePage };

/** A change that asks first, as the classic page's `turbo_confirm` does. */
interface Pending {
  readonly person: Person;
  readonly kind: "reset" | "link" | "unlink" | "remove";
}

const CONFIRM_LABEL = {
  reset: "Reset two-step sign-in",
  link: "Allow Google sign-in",
  unlink: "Unlink Google sign-in",
  remove: "Remove",
} as const;

function confirmation(pending: Pending): string {
  switch (pending.kind) {
    case "reset":
      return twoFactorResetConfirmation(pending.person);
    case "link":
      return googleLinkConfirmation(pending.person);
    case "unlink":
      return googleUnlinkConfirmation(pending.person);
    case "remove":
      return REMOVE_CONFIRMATION;
  }
}

/** One person: their avatar and name, and for an administrator the classic row's buttons. */
function PersonRow({
  person,
  canAdminister,
  busy,
  onRole,
  onAsk,
}: {
  readonly person: Person;
  readonly canAdminister: boolean;
  readonly busy: boolean;
  readonly onRole: (person: Person) => void;
  readonly onAsk: (pending: Pending) => void;
}) {
  const administrator = person.role === "administrator";

  return (
    <li className="settings-list-row admin-person" data-banned={person.banned || undefined}>
      <Avatar name={person.name} userId={person.id} src={person.avatarUrl} size={32} />
      <span className="settings-list-main">
        <strong>
          {person.name}
          {administrator ? <span className="settings-badge">Administrator</span> : null}
          {person.banned ? <span className="settings-badge">Banned</span> : null}
        </strong>
        {person.emailAddress === null ? null : (
          <span className="text-faint">{person.emailAddress}</span>
        )}
      </span>
      <span className="admin-person-actions">
        {person.you ? (
          <Link to="/settings" className="button" data-variant="secondary" data-size="sm">
            My settings
          </Link>
        ) : null}
        {canAdminister && !person.banned ? (
          <>
            {!person.you && person.twoFactorEnabled ? (
              <Button
                variant="ghost"
                size="sm"
                icon="lock-open"
                disabled={busy}
                title="Reset two-step sign-in"
                onClick={() => onAsk({ person, kind: "reset" })}
              >
                <span className="visually-hidden">Reset two-step sign-in for {person.name}</span>
              </Button>
            ) : null}
            <Button
              variant="ghost"
              size="sm"
              role="switch"
              aria-checked={administrator}
              disabled={busy || person.you}
              title={`Role: ${administrator ? "Administrator" : "Member"}`}
              onClick={() => onRole(person)}
            >
              {administrator ? "Administrator" : "Member"}
              <span className="visually-hidden">
                {" "}
                (make {person.name} {administrator ? "a member" : "an administrator"})
              </span>
            </Button>
            {person.googleIdentityEmail !== null ? (
              <Button
                variant="ghost"
                size="sm"
                icon="link"
                disabled={busy}
                title={googleUnlinkTitle(person.googleIdentityEmail)}
                onClick={() => onAsk({ person, kind: "unlink" })}
              >
                <span className="visually-hidden">Unlink Google sign-in from {person.name}</span>
              </Button>
            ) : person.offerGoogleEmailLink ? (
              <Button
                variant="secondary"
                size="sm"
                icon="link"
                disabled={busy}
                title={googleLinkTitle(person)}
                onClick={() => onAsk({ person, kind: "link" })}
              >
                <span className="visually-hidden">Allow Google sign-in for {person.name}</span>
              </Button>
            ) : null}
            {person.you ? null : (
              <Button
                variant="danger"
                size="sm"
                icon="trash"
                disabled={busy}
                onClick={() => onAsk({ person, kind: "remove" })}
              >
                <span className="visually-hidden">Remove {person.name}</span>
              </Button>
            )}
          </>
        ) : null}
      </span>
    </li>
  );
}

/**
 * People: everyone in the workspace, administrators first, as the classic account page lists them
 * (administrators also see banned people). An administrator changes roles, resets two-step
 * sign-in, allows or unlinks Google sign-in and removes people here; the changes the classic page
 * guards with the password ask for it first.
 */
export function PeopleSection() {
  const { workspace } = useAdmin();
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [pending, setPending] = useState<Pending | null>(null);
  const [more, setMore] = useState(false);
  const { busy, track } = useBusy();

  const fetchPeople = useCallback(() => {
    admin.people().then(
      (page) => setLoad({ status: "ready", page }),
      (error: Error) => setLoad({ status: "error", message: error.message }),
    );
  }, []);

  useEffect(fetchPeople, [fetchPeople]);

  const reload = () => {
    setLoad({ status: "loading" });
    fetchPeople();
  };

  const edit = (change: (people: readonly Person[]) => readonly Person[]) =>
    setLoad((current) =>
      current.status === "ready"
        ? { status: "ready", page: { ...current.page, people: [...change(current.page.people)] } }
        : current,
    );

  const landed = ({
    person,
    notice,
  }: {
    readonly person: Person;
    readonly notice: string | null;
  }) => {
    edit((people) => people.map((each) => (each.id === person.id ? person : each)));

    if (notice !== null) {
      toast({ title: notice, tone: "success" });
    }
  };

  const loadMore = (nextPage: string) => {
    setMore(true);
    admin
      .people(nextPage)
      .then(
        (page) =>
          setLoad((current) =>
            current.status === "ready"
              ? {
                  status: "ready",
                  page: {
                    people: [...current.page.people, ...page.people],
                    nextPage: page.nextPage,
                  },
                }
              : current,
          ),
        (error: Error) => adminFailure("Couldn't load more people", error),
      )
      .finally(() => setMore(false));
  };

  const changeRole = (person: Person) => {
    const role = person.role === "administrator" ? "member" : "administrator";

    void track(
      `person-${person.id}`,
      admin
        .setRole(person.id, role)
        .then(landed, (error: Error) =>
          adminFailure(`Couldn't change ${person.name}'s role`, error),
        ),
    );
  };

  const confirm = () => {
    const action = pending;

    setPending(null);

    if (action === null) {
      return;
    }

    const { person } = action;
    const key = `person-${person.id}`;

    switch (action.kind) {
      case "reset":
        void track(
          key,
          admin
            .resetTwoFactor(person.id)
            .then(landed, (error: Error) => adminFailure("Couldn't reset two-step sign-in", error)),
        );

        return;
      case "link":
      case "unlink":
        void track(
          key,
          admin
            .setGoogleLink(person.id, action.kind === "link")
            .then(landed, (error: Error) => adminFailure("Couldn't change Google sign-in", error)),
        );

        return;
      case "remove":
        void track(
          key,
          admin.removePerson(person.id).then(
            ({ id }) => {
              edit((people) => people.filter((each) => each.id !== id));
              toast({ title: `${person.name} was removed`, tone: "success" });
            },
            (error: Error) => adminFailure(`Couldn't remove ${person.name}`, error),
          ),
        );

        return;
    }
  };

  const rows = (people: readonly Person[]) =>
    people.map((person) => (
      <PersonRow
        key={person.id}
        person={person}
        canAdminister={workspace.canAdminister}
        busy={busy(`person-${person.id}`)}
        onRole={changeRole}
        onAsk={setPending}
      />
    ));

  const groups = load.status === "ready" ? groupPeople(load.page.people) : null;

  return (
    <SettingsPage title="People" description="Everyone in this workspace.">
      {load.status === "loading" ? <PaneListSkeleton rows={4} /> : null}
      {load.status === "error" ? <PaneError message={load.message} onRetry={reload} /> : null}
      {groups === null ? null : (
        <>
          {groups.administrators.length > 0 ? (
            <SettingsGroup title="Administrators">
              <ul className="settings-list">{rows(groups.administrators)}</ul>
            </SettingsGroup>
          ) : null}
          {groups.members.length > 0 ? (
            <SettingsGroup title="Members">
              <ul className="settings-list">{rows(groups.members)}</ul>
            </SettingsGroup>
          ) : null}
        </>
      )}
      {load.status === "ready" && load.page.nextPage !== null ? (
        <div className="settings-actions">
          <Button
            variant="secondary"
            loading={more}
            disabled={more}
            onClick={() => {
              if (load.page.nextPage !== null) loadMore(load.page.nextPage);
            }}
          >
            Show more people
          </Button>
        </div>
      ) : null}
      <Dialog
        open={pending !== null}
        onOpenChange={(open) => {
          if (!open) setPending(null);
        }}
        role="alertdialog"
        size="sm"
        title={pending === null ? "" : `${CONFIRM_LABEL[pending.kind]}?`}
        description={pending === null ? undefined : confirmation(pending)}
        footer={
          <>
            <Button variant="secondary" onClick={() => setPending(null)} data-autofocus>
              Cancel
            </Button>
            <Button variant={pending?.kind === "remove" ? "danger" : "primary"} onClick={confirm}>
              {pending === null ? "OK" : CONFIRM_LABEL[pending.kind]}
            </Button>
          </>
        }
      />
    </SettingsPage>
  );
}
