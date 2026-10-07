import { type FormEvent, useCallback, useEffect, useRef, useState } from "react";
import type { Credential } from "../../gen/Credential.ts";
import type { CredentialList } from "../../gen/CredentialList.ts";
import { bots } from "../../sync/admin.ts";
import { Button } from "../../ui/button.tsx";
import { TextField } from "../../ui/text-field.tsx";
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
import { adminFailure, keepDraft, needsSudo, takeDraft, useFocusAfter } from "./admin-parts.tsx";
import { CREDENTIAL_STATE, CREDENTIAL_USAGE, optional } from "./bot-format.ts";
import { BotBack, SecretDialog, useBotId } from "./bot-parts.tsx";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly list: CredentialList };

type Fields = Readonly<Record<string, readonly string[]>>;

/** Where the issue form waits while the classic page confirms the password. */
function draftKey(botId: number): string {
  return `smartfire.draft.admin-credential-${botId}`;
}

/** The issue form: a name and an optional expiry in the viewer's time zone. */
function NewCredential({
  botId,
  onIssued,
}: {
  readonly botId: number;
  readonly onIssued: (list: CredentialList, secret: string) => void;
}) {
  const [name, setName] = useState("");
  const [expiresAt, setExpiresAt] = useState("");
  const [fields, setFields] = useState<Fields>({});
  const { busy, track } = useBusy();
  // Whether the kept draft was taken: once, even when Strict Mode runs the effect twice.
  const taken = useRef(false);

  useEffect(() => {
    if (taken.current) {
      return;
    }

    taken.current = true;

    const kept = takeDraft(draftKey(botId));

    if (kept === null) {
      return;
    }

    const draft = new URLSearchParams(kept);

    setName(draft.get("name") ?? "");
    setExpiresAt(draft.get("expiresAt") ?? "");
    toast({ title: "Your credential is back", description: "Issue it to keep it." });
  }, [botId]);

  const submit = (event: FormEvent) => {
    event.preventDefault();
    setFields({});
    void track(
      "issue",
      bots.issueCredential(botId, { name, expiresAt: optional(expiresAt) }).then(
        ({ secret, credentials }) => {
          onIssued(credentials, secret);
          setName("");
          setExpiresAt("");
        },
        (error: Error) => {
          const named = fieldsOf(error);

          if (Object.keys(named).length > 0) {
            setFields(named);

            return;
          }

          if (needsSudo(error)) {
            keepDraft(draftKey(botId), new URLSearchParams({ name, expiresAt }).toString());
          }

          adminFailure("Couldn't issue the credential", error);
        },
      ),
    );
  };

  const first = (key: string) => fields[key]?.[0];

  return (
    <form className="settings-form" onSubmit={submit}>
      <FieldError message={first("base")} />
      <div className="settings-inline admin-icon-names">
        <TextField
          label="Name"
          value={name}
          required
          autoComplete="off"
          placeholder="Credential name"
          error={first("name")}
          onChange={(event) => setName(event.target.value)}
        />
        <TextField
          label="Optional expiry"
          type="datetime-local"
          value={expiresAt}
          error={first("expiresAt")}
          onChange={(event) => setExpiresAt(event.target.value)}
        />
      </div>
      <div className="settings-actions">
        <Button type="submit" variant="primary" loading={busy("issue")} disabled={busy("issue")}>
          Issue credential
        </Button>
      </div>
    </form>
  );
}

/** One credential: its name, last four characters, who made it and when, and where it stands. */
function CredentialRow({
  credential,
  busy,
  onRevoke,
}: {
  readonly credential: Credential;
  readonly busy: boolean;
  readonly onRevoke: (credential: Credential) => void;
}) {
  return (
    <li className="settings-list-row" data-row={credential.id} tabIndex={-1}>
      <span className="settings-list-main">
        <strong>
          {credential.name} <code className="admin-code">…{credential.lastFour}</code>
          {credential.state === "active" ? null : (
            <span className="settings-badge">{CREDENTIAL_STATE[credential.state]}</span>
          )}
        </strong>
        <span className="text-faint">
          By {credential.createdBy}, {auditTime(credential.createdAt)}
          {credential.lastUsedAt === null
            ? " · never used"
            : ` · last used ${auditTime(credential.lastUsedAt)}`}
          {credential.expiresAt === null ? "" : ` · expires ${credential.expiresAt}`}
        </span>
      </span>
      {credential.state === "revoked" ? null : (
        <Button
          variant="danger"
          size="sm"
          data-row-control="revoke"
          disabled={busy}
          onClick={() => onRevoke(credential)}
        >
          Revoke
          <span className="visually-hidden"> {credential.name}</span>
        </Button>
      )}
    </li>
  );
}

/**
 * A bot's credentials: bearer tokens for the agent API, newest first. Administrators issue them
 * (the secret shows once); the agent's owner may revoke them. A legacy bot gets its agent here,
 * as on the classic page.
 */
export function BotCredentialsSection() {
  const botId = useBotId();
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [secret, setSecret] = useState<string | null>(null);
  const { busy, track } = useBusy();
  const { container, focusAfter } = useFocusAfter();

  const fetchCredentials = useCallback(() => {
    bots.credentials(botId).then(
      (list) => setLoad({ status: "ready", list }),
      (error: Error) => setLoad({ status: "error", message: error.message }),
    );
  }, [botId]);

  useEffect(fetchCredentials, [fetchCredentials]);

  const reload = () => {
    setLoad({ status: "loading" });
    fetchCredentials();
  };

  const revoke = (credential: Credential) =>
    void track(
      `credential-${credential.id}`,
      bots.revokeCredential(botId, credential.id).then(
        (list) => {
          setLoad({ status: "ready", list });
          // Its Revoke button goes away; the row keeps the focus.
          focusAfter({ row: `${credential.id}`, control: "revoke" });
          toast({ title: `${credential.name} was revoked`, tone: "success" });
        },
        (error: Error) => adminFailure(`Couldn't revoke ${credential.name}`, error),
      ),
    );

  const list = load.status === "ready" ? load.list : null;

  return (
    <SettingsPage
      title={list === null ? "Credentials" : `${list.botName}'s credentials`}
      description="Bearer tokens for API access. The secret is shown once at creation."
    >
      <BotBack botId={botId} label="Back to the bot" />
      {list === null ? null : (
        <SettingsGroup title="New credential">
          {list.canIssue ? (
            <NewCredential
              botId={botId}
              onIssued={(next, issued) => {
                setLoad({ status: "ready", list: next });
                setSecret(issued);
              }}
            />
          ) : (
            <p className="text-muted">
              Only an administrator can issue credentials. You can revoke them below.
            </p>
          )}
        </SettingsGroup>
      )}
      <SettingsGroup title="Credentials">
        <div ref={container} tabIndex={-1} className="admin-focus-root">
          {load.status === "loading" ? <PaneListSkeleton rows={3} /> : null}
          {load.status === "error" ? <PaneError message={load.message} onRetry={reload} /> : null}
          {list !== null && list.credentials.length === 0 ? (
            <p className="text-muted">No credentials yet.</p>
          ) : null}
          {list !== null && list.credentials.length > 0 ? (
            <ul className="settings-list">
              {list.credentials.map((credential) => (
                <CredentialRow
                  key={credential.id}
                  credential={credential}
                  busy={busy(`credential-${credential.id}`)}
                  onRevoke={revoke}
                />
              ))}
            </ul>
          ) : null}
        </div>
      </SettingsGroup>
      <SecretDialog secret={secret} usage={CREDENTIAL_USAGE} onClose={() => setSecret(null)} />
    </SettingsPage>
  );
}
