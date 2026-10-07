import { type FormEvent, useCallback, useEffect, useId, useState } from "react";
import type { Grant } from "../../gen/Grant.ts";
import type { GrantList } from "../../gen/GrantList.ts";
import { bots } from "../../sync/admin.ts";
import { Button } from "../../ui/button.tsx";
import { toast } from "../../ui/toast-store.ts";
import { PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import {
  FieldError,
  fieldsOf,
  SettingsGroup,
  SettingsPage,
  useBusy,
} from "../settings/settings-parts.tsx";
import { auditTime } from "./admin-format.ts";
import { adminFailure, useFocusAfter } from "./admin-parts.tsx";
import { BotBack, useBotId } from "./bot-parts.tsx";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly list: GrantList };

/** The grant form: a capability, workspace-wide or in one of the bot's rooms. */
function NewGrant({
  list,
  onGranted,
}: {
  readonly list: GrantList;
  readonly onGranted: (list: GrantList) => void;
}) {
  const id = useId();
  const [capability, setCapability] = useState(list.capabilities[0] ?? "");
  const [room, setRoom] = useState("");
  const [error, setError] = useState<string | undefined>(undefined);
  const { busy, track } = useBusy();

  const submit = (event: FormEvent) => {
    event.preventDefault();
    setError(undefined);
    void track(
      "grant",
      bots.grant(list.botId, { capability, roomId: room === "" ? null : Number(room) }).then(
        (next) => {
          onGranted(next);
          toast({ title: `Granted ${capability}`, tone: "success" });
        },
        (failure: Error) => {
          const named = Object.values(fieldsOf(failure)).flat();

          if (named.length > 0) {
            setError(named.join(" "));
          } else {
            adminFailure("Couldn't grant the capability", failure);
          }
        },
      ),
    );
  };

  return (
    <form className="settings-form" onSubmit={submit}>
      <div className="settings-inline admin-icon-names">
        <div className="settings-field">
          <label className="settings-label" htmlFor={`${id}-capability`}>
            Capability
          </label>
          <select
            id={`${id}-capability`}
            className="input settings-select"
            required
            value={capability}
            onChange={(event) => setCapability(event.target.value)}
          >
            {list.capabilities.map((each) => (
              <option key={each} value={each}>
                {each}
              </option>
            ))}
          </select>
        </div>
        <div className="settings-field">
          <label className="settings-label" htmlFor={`${id}-room`}>
            Room
          </label>
          <select
            id={`${id}-room`}
            className="input settings-select"
            value={room}
            onChange={(event) => setRoom(event.target.value)}
          >
            <option value="">Workspace-wide</option>
            {list.rooms.map((each) => (
              <option key={each.id} value={`${each.id}`}>
                {each.name}
              </option>
            ))}
          </select>
          <p className="settings-hint text-faint">Room membership still applies</p>
        </div>
      </div>
      <FieldError message={error} />
      <div className="settings-actions">
        <Button type="submit" variant="primary" loading={busy("grant")} disabled={busy("grant")}>
          Grant capability
        </Button>
      </div>
    </form>
  );
}

/** One grant: the capability, where it applies, who gave it and when. */
function GrantRow({
  grant,
  busy,
  onRevoke,
}: {
  readonly grant: Grant;
  readonly busy: boolean;
  readonly onRevoke: (grant: Grant) => void;
}) {
  return (
    <li className="settings-list-row" data-row={grant.id} tabIndex={-1}>
      <span className="settings-list-main">
        <strong>
          <code className="admin-code">{grant.capability}</code> · {grant.roomName}
          {grant.revoked ? <span className="settings-badge">Revoked</span> : null}
        </strong>
        <span className="text-faint">
          By {grant.grantedBy}, {auditTime(grant.createdAt)}
        </span>
      </span>
      {grant.revoked ? null : (
        <Button
          variant="danger"
          size="sm"
          data-row-control="revoke"
          disabled={busy}
          onClick={() => onRevoke(grant)}
        >
          Revoke
          <span className="visually-hidden">
            {" "}
            {grant.capability} ({grant.roomName})
          </span>
        </Button>
      )}
    </li>
  );
}

/**
 * A bot's grants: the capabilities its agent holds, active first. Administrators grant them; the
 * agent's owner may revoke them. With no grant ever made, the agent keeps its legacy access.
 */
export function BotGrantsSection() {
  const botId = useBotId();
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const { busy, track } = useBusy();
  const { container, focusAfter } = useFocusAfter();

  const fetchGrants = useCallback(() => {
    bots.grants(botId).then(
      (list) => setLoad({ status: "ready", list }),
      (error: Error) => setLoad({ status: "error", message: error.message }),
    );
  }, [botId]);

  useEffect(fetchGrants, [fetchGrants]);

  const reload = () => {
    setLoad({ status: "loading" });
    fetchGrants();
  };

  const revoke = (grant: Grant) =>
    void track(
      `grant-${grant.id}`,
      bots.revokeGrant(botId, grant.id).then(
        (list) => {
          setLoad({ status: "ready", list });
          focusAfter({ row: `${grant.id}`, control: "revoke" });
          toast({ title: `Revoked ${grant.capability}`, tone: "success" });
        },
        (error: Error) => adminFailure(`Couldn't revoke ${grant.capability}`, error),
      ),
    );

  const list = load.status === "ready" ? load.list : null;

  return (
    <SettingsPage
      title={list === null ? "Grants" : `${list.botName}'s grants`}
      description="Every capability is enforced."
    >
      <BotBack botId={botId} label="Back to the bot" />
      {list?.legacy === true ? (
        <p className="settings-callout" role="status">
          Legacy access: with no grants ever created, this agent can read and post in rooms it
          belongs to.
        </p>
      ) : null}
      {list === null ? null : (
        <SettingsGroup title="New grant">
          {list.canGrant ? (
            <NewGrant list={list} onGranted={(next) => setLoad({ status: "ready", list: next })} />
          ) : (
            <p className="text-muted">
              Only an administrator can grant capabilities. You can revoke grants below.
            </p>
          )}
        </SettingsGroup>
      )}
      <SettingsGroup title="Grants">
        <div ref={container} tabIndex={-1} className="admin-focus-root">
          {load.status === "loading" ? <PaneListSkeleton rows={3} /> : null}
          {load.status === "error" ? <PaneError message={load.message} onRetry={reload} /> : null}
          {list !== null && list.grants.length === 0 ? (
            <p className="text-muted">No grants yet.</p>
          ) : null}
          {list !== null && list.grants.length > 0 ? (
            <ul className="settings-list">
              {list.grants.map((grant) => (
                <GrantRow
                  key={grant.id}
                  grant={grant}
                  busy={busy(`grant-${grant.id}`)}
                  onRevoke={revoke}
                />
              ))}
            </ul>
          ) : null}
        </div>
      </SettingsGroup>
    </SettingsPage>
  );
}
