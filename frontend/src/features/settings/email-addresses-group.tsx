import { type FormEvent, useCallback, useEffect, useRef, useState } from "react";
import type { EmailAddressEntry } from "../../gen/EmailAddressEntry.ts";
import type { EmailAddressList } from "../../gen/EmailAddressList.ts";
import { ActionError } from "../../sync/run.ts";
import { settings as settingsActions } from "../../sync/settings.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import { PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import { fieldError } from "./settings-format.ts";
import { SettingsGroup, toastFailure, useBusy } from "./settings-parts.tsx";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly list: EmailAddressList };

/** The add form's problem with the address: the server's field message, or its rate limit. */
function addProblem(error: Error): string | undefined {
  if (!(error instanceof ActionError)) return undefined;

  if (error.tag === "RateLimited") return error.message;

  if (error.tag === "Validation") {
    return fieldError(error.fields, "emailAddress", "Email address") ?? error.message;
  }

  return undefined;
}

/** Adds an address by emailing it a link; says the same whether or not the address is free. */
function AddEmailAddress({ onSent }: { readonly onSent: () => void }) {
  const [address, setAddress] = useState("");
  const [error, setError] = useState<string | undefined>(undefined);
  const [attempt, setAttempt] = useState(0);
  const [saving, setSaving] = useState(false);
  const [sent, setSent] = useState<string | null>(null);

  const submit = (event: FormEvent) => {
    event.preventDefault();

    if (saving) return;

    setSaving(true);
    settingsActions
      .addEmailAddress(address)
      .then(
        (result) => {
          setAddress("");
          setError(undefined);
          setSent(result.emailAddress);
          onSent();
        },
        (failure: Error) => {
          const problem = addProblem(failure);

          if (problem === undefined) {
            toastFailure("Couldn't add that email address", failure);
          } else {
            setError(problem);
            setAttempt((count) => count + 1);
          }
        },
      )
      .finally(() => setSaving(false));
  };

  return (
    <>
      <form className="settings-token settings-email-add" onSubmit={submit} noValidate>
        <TextField
          label="Add email address"
          type="email"
          value={address}
          autoComplete="email"
          spellCheck={false}
          placeholder="you@example.com"
          error={error}
          attempt={attempt}
          onChange={(event) => setAddress(event.target.value)}
        />
        <Button
          type="submit"
          variant="primary"
          size="lg"
          loading={saving}
          loadingLabel="Sending…"
          disabled={address.trim() === ""}
        >
          Send link
        </Button>
      </form>
      {sent === null ? null : (
        <p className="settings-callout settings-email-address" role="status">
          If {sent} isn't already used by an account, we've emailed it a confirmation link. Open the
          link within 24 hours to finish linking.
        </p>
      )}
    </>
  );
}

/** A linked address, with the ways to make it primary or remove it. */
function LinkedRow({
  entry,
  busy,
  onMakePrimary,
  onRemove,
}: {
  readonly entry: EmailAddressEntry;
  readonly busy: boolean;
  readonly onMakePrimary: () => void;
  readonly onRemove: () => void;
}) {
  return (
    <li className="settings-list-row settings-email-row">
      <span className="settings-list-main settings-email-address">{entry.emailAddress}</span>
      <span className="settings-email-actions">
        <Button
          variant="secondary"
          size="sm"
          disabled={busy}
          aria-label={`Make ${entry.emailAddress} primary`}
          onClick={onMakePrimary}
        >
          Make primary
        </Button>
        <Button
          variant="secondary"
          size="sm"
          disabled={busy}
          aria-label={`Remove ${entry.emailAddress}`}
          onClick={onRemove}
        >
          Remove
        </Button>
      </span>
    </li>
  );
}

/**
 * Email addresses: the primary, the confirmed addresses linked besides it (each signs in and finds
 * the account like the primary; notifications still go to the primary), and adding one by emailed
 * link. Each write may stop for the app's fresh confirmation first. `onPrimaryChanged` runs after
 * another address becomes primary, so the page can show the new one.
 */
export function EmailAddressesGroup({
  onPrimaryChanged,
}: {
  readonly onPrimaryChanged: () => void;
}) {
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [removing, setRemoving] = useState<EmailAddressEntry | null>(null);

  const [promoting, setPromoting] = useState<{
    readonly entry: EmailAddressEntry;
    readonly previous: string | null;
  } | null>(null);

  const { busy, track } = useBusy();
  const listRequest = useRef(0);
  const canAdd = load.status === "ready" && load.list.canAdd;

  const fetchList = useCallback(() => {
    const request = ++listRequest.current;

    settingsActions.emailAddresses().then(
      (list) => {
        if (request === listRequest.current) setLoad({ status: "ready", list });
      },
      (error: Error) => {
        if (request === listRequest.current) setLoad({ status: "error", message: error.message });
      },
    );
  }, []);

  useEffect(fetchList, [fetchList]);

  const reload = () => {
    setLoad({ status: "loading" });
    fetchList();
  };

  const makePrimary = () => {
    const entry = promoting?.entry;

    setPromoting(null);

    if (entry === undefined) return;

    void track(`address-${entry.id}`, settingsActions.setPrimaryEmailAddress(entry.id)).then(
      () => {
        toast({ title: `${entry.emailAddress} is now your primary address`, tone: "success" });
        fetchList();
        onPrimaryChanged();
      },
      (error: Error) => {
        toastFailure("Couldn't change your primary address", error);
        fetchList();
      },
    );
  };

  const remove = () => {
    const entry = removing;

    setRemoving(null);

    if (entry === null) return;

    void track(`address-${entry.id}`, settingsActions.removeEmailAddress(entry.id)).then(
      () => {
        toast({ title: `Removed ${entry.emailAddress}`, tone: "success" });
        fetchList();
      },
      (error: Error) => {
        toastFailure("Couldn't remove that email address", error);
        fetchList();
      },
    );
  };

  return (
    <SettingsGroup
      title="Email addresses"
      description={`Sign in or reset your password with any of these. Notifications go to your primary address. ${
        canAdd
          ? "If you switch your primary address, we'll email the old one a link to link it again."
          : "If you switch your primary address, the old one will be unlinked."
      }`}
    >
      {load.status === "loading" ? <PaneListSkeleton rows={2} /> : null}
      {load.status === "error" ? <PaneError message={load.message} onRetry={reload} /> : null}
      {load.status === "ready" ? (
        <>
          <ul className="settings-list" aria-label="Your email addresses">
            {load.list.primary === null ? null : (
              <li className="settings-list-row settings-email-row">
                <span className="settings-list-main settings-email-address">
                  <span>
                    {load.list.primary}
                    <span className="settings-badge">Primary</span>
                  </span>
                </span>
              </li>
            )}
            {load.list.secondary.map((entry) => (
              <LinkedRow
                key={entry.id}
                entry={entry}
                busy={busy(`address-${entry.id}`)}
                onMakePrimary={() => setPromoting({ entry, previous: load.list.primary })}
                onRemove={() => setRemoving(entry)}
              />
            ))}
          </ul>
          {load.list.secondary.length === 0 ? (
            <p className="settings-hint text-faint">No other addresses linked yet.</p>
          ) : null}
          {canAdd ? (
            <AddEmailAddress onSent={fetchList} />
          ) : (
            <p className="settings-hint text-faint">
              This workspace can't email confirmation links, so you can't add another address.
            </p>
          )}
        </>
      ) : null}
      <Dialog
        open={promoting !== null}
        onOpenChange={(open) => {
          if (!open) setPromoting(null);
        }}
        role="alertdialog"
        size="sm"
        title={`Make ${promoting?.entry.emailAddress ?? "this address"} your primary address?`}
        description={
          promoting === null ? null : (
            <span className="settings-email-address">
              Notifications will go to {promoting.entry.emailAddress}.
              {promoting.previous === null ? null : (
                <>
                  {` ${promoting.previous} will be unlinked`}
                  {canAdd ? ", and we'll email it a link to link it again." : "."}
                </>
              )}
            </span>
          )
        }
        footer={
          <>
            <Button variant="secondary" onClick={() => setPromoting(null)} data-autofocus>
              Cancel
            </Button>
            <Button variant="primary" onClick={makePrimary}>
              Make primary
            </Button>
          </>
        }
      />
      <Dialog
        open={removing !== null}
        onOpenChange={(open) => {
          if (!open) setRemoving(null);
        }}
        role="alertdialog"
        size="sm"
        title="Remove this email address?"
        description={
          <span className="settings-email-address">
            You won't be able to use {removing?.emailAddress ?? "it"} to sign in or reset your
            password.
          </span>
        }
        footer={
          <>
            <Button variant="secondary" onClick={() => setRemoving(null)} data-autofocus>
              Cancel
            </Button>
            <Button variant="danger" onClick={remove}>
              Remove
            </Button>
          </>
        }
      />
    </SettingsGroup>
  );
}
