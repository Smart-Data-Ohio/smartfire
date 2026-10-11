import { type FormEvent, useEffect, useRef, useState, useSyncExternalStore } from "react";
import type { SudoMethod } from "../../gen/SudoMethod.ts";
import { actions, type ConfirmationPrompt, confirmationGate } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { TextField } from "../../ui/text-field.tsx";
import "./reauthenticate.css";

/** What the dialog is sending: one credential at a time, or the start of a Google confirmation. */
type Busy = "password" | "totp" | "google" | null;

/** The classic page's heading and lede. */
const TITLE = "Confirm it's you";

const LEDE = "This action needs a fresh confirmation. Confirm once and it continues.";

/** The Google "G", as the classic page draws it beside "Confirm with Google". */
function GoogleMark() {
  return (
    <svg
      className="reauth-google-mark"
      width="18"
      height="18"
      viewBox="0 0 48 48"
      aria-hidden="true"
      focusable="false"
    >
      <path
        fill="#EA4335"
        d="M24 9.5c3.54 0 6.71 1.22 9.21 3.6l6.85-6.85C35.9 2.38 30.47 0 24 0 14.62 0 6.51 5.38 2.56 13.22l7.98 6.19C12.43 13.72 17.74 9.5 24 9.5z"
      />
      <path
        fill="#4285F4"
        d="M46.98 24.55c0-1.57-.15-3.09-.38-4.55H24v9.02h12.94c-.58 2.96-2.26 5.48-4.78 7.18l7.73 6c4.51-4.18 7.09-10.36 7.09-17.65z"
      />
      <path
        fill="#FBBC05"
        d="M10.53 28.59c-.48-1.45-.76-2.99-.76-4.59s.27-3.14.76-4.59l-7.98-6.19C.92 16.46 0 20.12 0 24c0 3.88.92 7.54 2.56 10.78l7.97-6.19z"
      />
      <path
        fill="#34A853"
        d="M24 48c6.48 0 11.93-2.13 15.89-5.81l-7.73-6c-2.15 1.45-4.92 2.3-8.16 2.3-6.26 0-11.57-4.22-13.47-9.91l-7.98 6.19C6.51 42.62 14.62 48 24 48z"
      />
    </svg>
  );
}

/**
 * The fresh confirmation a guarded write asks for, in place of the classic "Confirm it's you"
 * page: the password, an authenticator code, or Google, whichever the account has. Confirming
 * replays the waiting writes; Cancel (Esc, the close button) fails them unsent. Mounted once by
 * the shell; it opens whenever a write is held (src/sync/reauthentication.ts).
 */
export function ReauthenticateDialog() {
  const prompt = useSyncExternalStore(confirmationGate.subscribe, confirmationGate.snapshot);
  const [busy, setBusy] = useState<Busy>(null);
  // Leaving for Google settles the writes at once; the dialog stays up until the page goes.
  const open = prompt !== null || busy === "google";
  // What it shows, kept while the exit plays once the writes have settled.
  const [shown, setShown] = useState<ConfirmationPrompt | null>(prompt);
  const [opened, setOpened] = useState(0);
  const [wasOpen, setWasOpen] = useState(open);
  const [dirty, setDirty] = useState(false);
  // The control that sent the write. A busy button disables itself, which drops focus to the page
  // before the dialog opens, so the dialog can't see it as its opener: it's taken from the last
  // focus at the moment a write is first held, and focus goes back to it on close.
  const sender = useRef<HTMLElement | null>(null);

  useEffect(() => {
    let lastFocus: HTMLElement | null = null;
    let held = confirmationGate.snapshot() !== null;

    const track = (event: FocusEvent) => {
      if (event.target instanceof HTMLElement) lastFocus = event.target;
    };

    const unsubscribe = confirmationGate.subscribe(() => {
      const holding = confirmationGate.snapshot() !== null;

      if (holding && !held) sender.current = lastFocus;
      held = holding;
    });

    document.addEventListener("focusin", track);

    return () => {
      unsubscribe();
      document.removeEventListener("focusin", track);
    };
  }, []);

  if (prompt !== null && prompt !== shown) {
    setShown(prompt);
  }

  if (open !== wasOpen) {
    setWasOpen(open);

    // Each opening starts with empty fields.
    if (open) {
      setOpened((count) => count + 1);
    }
  }

  const cancel = () => {
    if (busy === null) {
      actions.confirmation.cancel();
    }
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) cancel();
      }}
      title={TITLE}
      description={LEDE}
      size="sm"
      dirty={dirty}
      returnFocus={() => (sender.current?.isConnected === true ? sender.current : null)}
      onExited={() => setShown(null)}
      footer={
        <Button variant="secondary" disabled={busy !== null} onClick={cancel}>
          Cancel
        </Button>
      }
    >
      {shown === null ? null : (
        <Choices key={opened} prompt={shown} busy={busy} onBusy={setBusy} onDirty={setDirty} />
      )}
    </Dialog>
  );
}

interface ChoicesProps {
  readonly prompt: ConfirmationPrompt;
  readonly busy: Busy;
  readonly onBusy: (busy: Busy) => void;
  /** Whether a credential has been typed, so a swipe down doesn't throw it away. */
  readonly onDirty: (dirty: boolean) => void;
}

/** What each credential's field asks for when it's sent empty. */
const BLANK = {
  password: "Enter your password.",
  totp: "Enter the code from your authenticator app.",
} as const;

/** The account's ways to confirm, stacked with "or" between, as on the classic page. */
function Choices({ prompt, busy, onBusy, onDirty }: ChoicesProps) {
  const [password, setPassword] = useState("");
  const [code, setCode] = useState("");
  const [errors, setErrors] = useState<{ readonly password?: string; readonly totp?: string }>({});
  const [attempt, setAttempt] = useState(0);
  const [googleError, setGoogleError] = useState("");
  const passwordField = useRef<HTMLInputElement>(null);
  const codeField = useRef<HTMLInputElement>(null);
  const has = (method: SudoMethod) => prompt.methods.includes(method);

  useEffect(() => onDirty(password !== "" || code !== ""), [password, code, onDirty]);

  useEffect(() => () => onDirty(false), [onDirty]);

  const refuse = (kind: "password" | "totp", message: string) => {
    setErrors({ [kind]: message });
    setAttempt((count) => count + 1);

    if (kind === "password") {
      setPassword("");
      passwordField.current?.focus();
    } else {
      setCode("");
      codeField.current?.focus();
    }
  };

  const confirm = (kind: "password" | "totp") => (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    if (busy !== null) {
      return;
    }

    const value = kind === "password" ? password : code.trim();

    if (value === "") {
      refuse(kind, BLANK[kind]);

      return;
    }

    setErrors({});
    setGoogleError("");
    onBusy(kind);
    actions.confirmation
      .submit(kind === "password" ? { kind, password: value } : { kind, code: value })
      .then(
        (refusal) => {
          onBusy(null);

          if (refusal !== null) {
            refuse(kind, refusal);
          }
        },
        (failure: Error) => {
          onBusy(null);
          refuse(kind, failure.message);
        },
      );
  };

  const google = () => {
    if (busy !== null) {
      return;
    }

    setErrors({});
    setGoogleError("");
    onBusy("google");
    actions.confirmation.startGoogle(`${window.location.pathname}${window.location.search}`).then(
      (start) => {
        if (start.kind === "navigate") {
          window.location.assign(start.location);

          return;
        }

        onBusy(null);
        setGoogleError(start.message);
      },
      (failure: Error) => {
        onBusy(null);
        setGoogleError(failure.message);
      },
    );
  };

  const choices = [
    has("password") ? (
      <form key="password" className="reauth-form" onSubmit={confirm("password")} noValidate>
        <TextField
          ref={passwordField}
          label="Password"
          type="password"
          value={password}
          maxLength={72}
          autoComplete="current-password"
          data-autofocus
          error={errors.password}
          attempt={attempt}
          onChange={(event) => setPassword(event.target.value)}
        />
        <Button
          type="submit"
          variant="primary"
          className="reauth-wide"
          loading={busy === "password"}
          loadingLabel="Confirming"
        >
          Confirm password
        </Button>
      </form>
    ) : null,
    has("totp") ? (
      <form key="totp" className="reauth-form" onSubmit={confirm("totp")} noValidate>
        <TextField
          ref={codeField}
          label="Authenticator code"
          value={code}
          inputMode="numeric"
          maxLength={10}
          autoComplete="one-time-code"
          placeholder="Enter your authenticator code"
          data-autofocus={has("password") ? undefined : true}
          error={errors.totp}
          attempt={attempt}
          onChange={(event) => setCode(event.target.value)}
        />
        <Button
          type="submit"
          variant={has("password") ? "secondary" : "primary"}
          className="reauth-wide"
          loading={busy === "totp"}
          loadingLabel="Confirming"
        >
          Confirm code
        </Button>
      </form>
    ) : null,
    has("google") ? (
      <div key="google" className="reauth-google">
        <Button
          variant="secondary"
          className="reauth-wide"
          loading={busy === "google"}
          loadingLabel="Opening Google"
          onClick={google}
        >
          <GoogleMark />
          Confirm with Google
        </Button>
        {prompt.secrets > 0 ? (
          <p className="reauth-note">
            Google opens in this tab. A token or secret you entered here isn't kept, so enter it
            again when you're back.
          </p>
        ) : null}
        <p className="reauth-alert" role="alert">
          {googleError}
        </p>
      </div>
    ) : null,
  ].filter((choice) => choice !== null);

  if (choices.length === 0) {
    return (
      <p className="reauth-note">
        This account has no password, authenticator or Google sign-in to confirm with. Ask an
        administrator for help.
      </p>
    );
  }

  return (
    <div className="reauth">
      {choices.flatMap((choice, index) =>
        index === 0
          ? [choice]
          : [
              <p key={`or-${choice.key}`} className="reauth-or">
                or
              </p>,
              choice,
            ],
      )}
    </div>
  );
}
