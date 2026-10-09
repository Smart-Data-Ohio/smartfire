import { type FormEvent, useCallback, useEffect, useId, useState } from "react";
import type { AccountSettings } from "../../gen/AccountSettings.ts";
import type { RememberedDevice } from "../../gen/RememberedDevice.ts";
import type { TwoFactorChange } from "../../gen/TwoFactorChange.ts";
import type { TwoFactorSettings } from "../../gen/TwoFactorSettings.ts";
import { postClassicForm } from "../../lib/classic-form.ts";
import { ActionError } from "../../sync/run.ts";
import { settings as settingsActions } from "../../sync/settings.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import { PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import { useNow } from "../threads/use-now.ts";
import { reauthLabel, rememberedMeta, svgDataUrl, twoFactorSince } from "./settings-format.ts";
import { SettingsGroup, SettingsPage, toastFailure } from "./settings-parts.tsx";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly account: AccountSettings };

/** A two-step change waiting for its confirmation. */
type Ask =
  | { readonly kind: "codes" }
  | { readonly kind: "disable" }
  | { readonly kind: "forget"; readonly device: RememberedDevice }
  | { readonly kind: "forget-all" };

/** The sign-in link's warning, the classic share sheet's words. */
const LINK_WARNING =
  "This is your own private sign-in link. Don't share it. Use it to sign in on another device or if you get locked out.";

/** The two-step sign-in group, for putting focus back when a dialog's opener is gone. */
const TWO_FACTOR_ID = "settings-two-factor";

/** The sign-in link group, likewise. */
const TRANSFER_ID = "settings-transfer";

/**
 * Where focus lands when a two-step dialog closes and its opener has gone (a forgotten browser's
 * button, or the confirmation that made new codes): the first remaining Forget, else the group's
 * first button.
 */
function twoFactorControl(): HTMLElement | null {
  const group = document.getElementById(TWO_FACTOR_ID);

  return (
    group?.querySelector<HTMLElement>("[data-forget]") ??
    group?.querySelector<HTMLElement>("button, a[href]") ??
    null
  );
}

/** Where focus lands when the new codes close: the button that makes them, else the group. */
function codesControl(): HTMLElement | null {
  return (
    document.getElementById(TWO_FACTOR_ID)?.querySelector<HTMLElement>("[data-codes]") ??
    twoFactorControl()
  );
}

/** The sign-in link group's first button, likewise. */
function transferControl(): HTMLElement | null {
  return document.getElementById(TRANSFER_ID)?.querySelector<HTMLElement>("button") ?? null;
}

/** Whether a refusal belongs under the confirmation field: a wrong code, or too many tries. */
function fieldRefusal(error: Error): boolean {
  return (
    error instanceof ActionError && (error.tag === "Validation" || error.tag === "RateLimited")
  );
}

/**
 * Security: two-step sign-in (new backup codes, turning it off, forgetting remembered browsers,
 * each confirmed with a code or password as the classic profile asks) and the private link that
 * signs you in on another device. Setting two-step sign-in up stays on the classic setup page.
 */
export function SecuritySection() {
  const [load, setLoad] = useState<Load>({ status: "loading" });

  const fetchAccount = useCallback(() => {
    settingsActions.account().then(
      (account) => setLoad({ status: "ready", account }),
      (error: Error) => setLoad({ status: "error", message: error.message }),
    );
  }, []);

  useEffect(fetchAccount, [fetchAccount]);

  const reload = () => {
    setLoad({ status: "loading" });
    fetchAccount();
  };

  const replaceTwoFactor = (twoFactor: TwoFactorSettings) => {
    setLoad((current) =>
      current.status === "ready"
        ? { status: "ready", account: { ...current.account, twoFactor } }
        : current,
    );
  };

  return (
    <SettingsPage
      title="Security"
      description="Two-step sign-in, and a private link that signs you in on another device."
    >
      {load.status === "loading" ? <PaneListSkeleton rows={3} /> : null}
      {load.status === "error" ? <PaneError message={load.message} onRetry={reload} /> : null}
      {load.status === "ready" ? (
        <TwoFactorGroup
          twoFactor={load.account.twoFactor}
          onChange={replaceTwoFactor}
          onStale={reload}
        />
      ) : null}
      {load.status === "ready" ? (
        <TransferGroup url={load.account.transferUrl} qrSvg={load.account.transferQrSvg} />
      ) : null}
    </SettingsPage>
  );
}

interface TwoFactorGroupProps {
  readonly twoFactor: TwoFactorSettings;
  readonly onChange: (twoFactor: TwoFactorSettings) => void;
  /** The panel was out of date (two-step sign-in was turned off elsewhere): load it again. */
  readonly onStale: () => void;
}

function TwoFactorGroup({ twoFactor, onChange, onStale }: TwoFactorGroupProps) {
  const [ask, setAsk] = useState<Ask | null>(null);
  const [codes, setCodes] = useState<readonly string[] | null>(null);
  const now = useNow();

  if (twoFactor.confirmedAt === null) {
    return (
      <SettingsGroup id={TWO_FACTOR_ID} title="Two-step sign-in" description="Not set up yet.">
        <div className="settings-actions">
          <a className="button" data-variant="primary" href="/two_factor_setup">
            Set up two-step sign-in
          </a>
        </div>
      </SettingsGroup>
    );
  }

  const changed = (change: TwoFactorChange) => {
    toast({ title: change.notice, tone: "success" });
    onChange(change.twoFactor);
  };

  /** Runs the confirmed change; a refusal outside the field closes the dialog with a toast. */
  const confirm = (kind: Ask, reauth: string): Promise<void> => {
    switch (kind.kind) {
      case "codes":
        return settingsActions.newBackupCodes(reauth).then((reply) => {
          setAsk(null);
          setCodes(reply.codes);
        });
      case "disable":
        return settingsActions.disableTwoFactor(reauth).then(() => {
          window.location.assign("/two_factor_setup");
        });
      case "forget":
        return settingsActions.forgetDevices(kind.device.id, reauth).then((change) => {
          setAsk(null);
          changed(change);
        });
      case "forget-all":
        return settingsActions.forgetDevices(null, reauth).then((change) => {
          setAsk(null);
          changed(change);
        });
    }
  };

  const refused = (error: Error) => {
    setAsk(null);

    if (error instanceof ActionError && error.tag === "Conflict") {
      toast({ title: error.message });
      onStale();

      return;
    }

    toastFailure("Couldn't make that change", error);
  };

  return (
    <SettingsGroup
      id={TWO_FACTOR_ID}
      title="Two-step sign-in"
      description={`${twoFactorSince(twoFactor.confirmedAt)} Signing in asks for your authenticator code.`}
    >
      <p className="text-muted">
        New backup codes, turning it off and forgetting a browser ask for your{" "}
        {twoFactor.hasPassword ? "authenticator code or password" : "authenticator code"} first.
      </p>
      <div className="settings-actions">
        <Button variant="secondary" data-codes onClick={() => setAsk({ kind: "codes" })}>
          New backup codes
        </Button>
        <Button variant="danger" onClick={() => setAsk({ kind: "disable" })}>
          Turn off
        </Button>
        {twoFactor.google ? (
          <Button variant="ghost" onClick={() => postClassicForm("/two_factor_reauthentication")}>
            Confirm with Google
          </Button>
        ) : null}
      </div>

      <h3 className="settings-subtitle text-ui">Remembered browsers</h3>
      {twoFactor.devices.length === 0 ? (
        <p className="text-muted">
          None. Tick "Remember this device" when signing in to skip the code on that browser for 30
          days.
        </p>
      ) : (
        <>
          <ul className="settings-list">
            {twoFactor.devices.map((device) => {
              const meta = rememberedMeta(device, now);

              return (
                <li key={device.id} className="settings-list-row">
                  <span className="settings-list-main">
                    <strong className="settings-endpoint" title={device.description}>
                      {device.description}
                    </strong>
                    {meta === null ? null : <span className="text-faint">{meta}</span>}
                  </span>
                  <Button
                    variant="secondary"
                    size="sm"
                    data-forget
                    aria-label={`Forget ${device.description}`}
                    onClick={() => setAsk({ kind: "forget", device })}
                  >
                    Forget
                  </Button>
                </li>
              );
            })}
          </ul>
          {twoFactor.devices.length > 1 ? (
            <div className="settings-actions">
              <Button variant="danger" onClick={() => setAsk({ kind: "forget-all" })}>
                Forget all browsers
              </Button>
            </div>
          ) : null}
        </>
      )}

      <ReauthDialog
        ask={ask}
        twoFactor={twoFactor}
        onClose={() => setAsk(null)}
        onConfirm={confirm}
        onRefused={refused}
      />
      <CodesDialog codes={codes} onClose={() => setCodes(null)} />
    </SettingsGroup>
  );
}

/** A change's dialog words. */
interface AskText {
  readonly title: string;
  readonly description: string;
  readonly action: string;
  readonly danger: boolean;
}

/** Each change's dialog words. */
function askText(ask: Ask): AskText {
  switch (ask.kind) {
    case "codes":
      return {
        title: "New backup codes?",
        description: "Your old backup codes stop working.",
        action: "Make new codes",
        danger: false,
      };
    case "disable":
      return {
        title: "Turn off two-step sign-in?",
        description:
          "Signing in will only ask for your password or Google. You'll go to the setup page to turn it on again.",
        action: "Turn off",
        danger: true,
      };
    case "forget":
      return {
        title: "Forget this browser?",
        description: "It will ask for a code at next sign-in.",
        action: "Forget",
        danger: true,
      };
    case "forget-all":
      return {
        title: "Forget every browser?",
        description: "Every browser will ask for a code at next sign-in.",
        action: "Forget all",
        danger: true,
      };
  }
}

interface ReauthDialogProps {
  readonly ask: Ask | null;
  readonly twoFactor: TwoFactorSettings;
  readonly onClose: () => void;
  readonly onConfirm: (ask: Ask, reauth: string) => Promise<void>;
  readonly onRefused: (error: Error) => void;
}

/**
 * The confirmation for a two-step change: a code or password field (left empty right after
 * "Confirm with Google"). A wrong code stays in the dialog under the field, as the classic alert
 * would say it.
 */
function ReauthDialog({ ask, twoFactor, onClose, onConfirm, onRefused }: ReauthDialogProps) {
  const formId = useId();
  const [shown, setShown] = useState<Ask | null>(ask);
  const [reauth, setReauth] = useState("");
  const [error, setError] = useState<string | undefined>(undefined);
  const [attempt, setAttempt] = useState(0);
  const [saving, setSaving] = useState(false);

  // Keep the last request's words while the dialog fades out; start clean on a new one.
  if (ask !== null && ask !== shown) {
    setShown(ask);
    setReauth("");
    setError(undefined);
  }

  // A closed dialog keeps no code or password.
  if (ask === null && reauth !== "") {
    setReauth("");
  }

  const submit = (event: FormEvent) => {
    event.preventDefault();

    if (ask === null || saving) {
      return;
    }

    setSaving(true);
    // The code or password is sent once and never kept, whatever the answer.
    setReauth("");
    onConfirm(ask, reauth)
      .then(
        () => undefined,
        (failure: Error) => {
          if (fieldRefusal(failure)) {
            setError(failure.message);
            setAttempt((count) => count + 1);
          } else {
            onRefused(failure);
          }
        },
      )
      .finally(() => {
        // The field is read-only while the request is out, but nothing typed survives it.
        setReauth("");
        setSaving(false);
      });
  };

  const text = shown === null ? null : askText(shown);
  const label = reauthLabel(twoFactor.hasPassword);

  return (
    <Dialog
      open={ask !== null}
      onOpenChange={(open) => {
        if (!open && !saving) onClose();
      }}
      size="sm"
      dirty={reauth !== ""}
      returnFocus={twoFactorControl}
      title={text?.title ?? ""}
      description={text?.description}
      footer={
        <>
          <Button variant="secondary" onClick={onClose} disabled={saving}>
            Cancel
          </Button>
          <Button
            type="submit"
            form={formId}
            variant={text?.danger ? "danger" : "primary"}
            loading={saving}
          >
            {text?.action}
          </Button>
        </>
      }
    >
      <form id={formId} onSubmit={submit} noValidate>
        <TextField
          label={label}
          type="password"
          value={reauth}
          readOnly={saving}
          autoComplete="off"
          spellCheck={false}
          data-autofocus
          {...(twoFactor.google ? { hint: "Just confirmed with Google? Leave this empty." } : {})}
          error={error}
          attempt={attempt}
          onChange={(event) => setReauth(event.target.value)}
        />
      </form>
    </Dialog>
  );
}

/** The new backup codes, shown once, with ways to keep them. */
function CodesDialog({
  codes,
  onClose,
}: {
  readonly codes: readonly string[] | null;
  readonly onClose: () => void;
}) {
  // The codes stay for the fade-out only; once the dialog has closed nothing holds them.
  const [kept, setKept] = useState<readonly string[]>([]);

  if (codes !== null && codes !== kept) {
    setKept(codes);
  }

  const text = kept.join("\n");

  const copy = () => {
    navigator.clipboard.writeText(text).then(
      () => toast({ title: "Backup codes copied", tone: "success" }),
      (error: Error) => toastFailure("Couldn't copy the codes", error),
    );
  };

  const download = () => {
    const link = document.createElement("a");

    link.href = URL.createObjectURL(new Blob([`${text}\n`], { type: "text/plain" }));
    link.download = "backup-codes.txt";
    link.click();
    URL.revokeObjectURL(link.href);
  };

  return (
    <Dialog
      open={codes !== null}
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
      size="sm"
      returnFocus={codesControl}
      onExited={() => setKept([])}
      title="Your new backup codes"
      description="Each code works once, if you can't use your authenticator. Keep them somewhere safe: they won't be shown again."
      footer={
        <>
          <Button variant="secondary" icon="copy" onClick={copy}>
            Copy
          </Button>
          <Button variant="secondary" icon="download" onClick={download}>
            Download
          </Button>
          <Button onClick={onClose} data-autofocus>
            Done
          </Button>
        </>
      }
    >
      <ol className="settings-codes" aria-label="Backup codes">
        {kept.map((code) => (
          <li key={code}>
            <code>{code}</code>
          </li>
        ))}
      </ol>
    </Dialog>
  );
}

/** The private sign-in link: copy it, scan its QR code, or hand it to the share sheet. */
function TransferGroup({ url, qrSvg }: { readonly url: string; readonly qrSvg: string }) {
  const [qr, setQr] = useState(false);
  const fieldId = useId();
  const canShare = "share" in navigator;

  const copy = () => {
    navigator.clipboard.writeText(url).then(
      () => toast({ title: "Sign-in link copied", tone: "success" }),
      (error: Error) => toastFailure("Couldn't copy the link", error),
    );
  };

  const share = () => {
    navigator.share({ title: "Your sign-in link", text: LINK_WARNING, url }).catch(() => undefined);
  };

  return (
    <SettingsGroup
      id={TRANSFER_ID}
      title="Sign in on another device"
      description="Open this link on another device to sign in there without a password."
    >
      <div className="settings-field">
        <label htmlFor={fieldId} className="settings-label">
          Your sign-in link
        </label>
        <input
          id={fieldId}
          className="input settings-transfer"
          value={url}
          readOnly
          onFocus={(event) => event.currentTarget.select()}
        />
        <p className="settings-hint text-faint">{LINK_WARNING}</p>
      </div>
      <div className="settings-actions">
        <Button variant="secondary" icon="copy" onClick={copy}>
          Copy link
        </Button>
        <Button variant="secondary" onClick={() => setQr(true)}>
          Show QR code
        </Button>
        {canShare ? (
          <Button variant="secondary" icon="arrow-up-right" onClick={share}>
            Share
          </Button>
        ) : null}
      </div>
      <Dialog
        open={qr}
        onOpenChange={setQr}
        size="sm"
        returnFocus={transferControl}
        title="Scan to sign in"
        description="Point your phone's camera at the code. Don't show it to anyone else."
      >
        <img className="settings-qr" src={svgDataUrl(qrSvg)} alt="QR code for your sign-in link" />
      </Dialog>
    </SettingsGroup>
  );
}
