import { type FormEvent, useEffect, useId, useRef, useState } from "react";
import { type AuthNext, auth } from "../../sync/auth.ts";
import { Button } from "../../ui/button.tsx";
import { Checkbox } from "../../ui/checkbox.tsx";
import { TextField } from "../../ui/text-field.tsx";
import {
  AuthHeader,
  AuthScreen,
  refusalMessage,
  useDocumentTitle,
  useFollowNext,
} from "./auth-parts.tsx";

type Method = "totp" | "recoveryCode";

/** The code field for each method: what it asks for and how a keyboard should offer it. */
const FIELDS = {
  totp: {
    label: "Authenticator code",
    placeholder: "123 456",
    inputMode: "numeric",
    autoComplete: "one-time-code",
    switchTo: "Use a backup code instead",
  },
  recoveryCode: {
    label: "Backup code",
    placeholder: "Enter a backup code",
    inputMode: "text",
    autoComplete: "off",
    switchTo: "Use your authenticator app instead",
  },
} as const;

/**
 * `/app/two_factor/challenge`: the retained second step (two_factor/challenges/show.html) in the
 * SPA. An authenticator code, or a backup code behind a switch, and "remember this device". A
 * visitor with no pending sign-in is sent back to sign in, as the retained page sends them.
 */
export function ChallengePage() {
  const titleId = useId();
  const follow = useFollowNext();
  const codeRef = useRef<HTMLInputElement | null>(null);
  const [methods, setMethods] = useState<readonly Method[]>(["totp", "recoveryCode"]);
  const [rememberOffered, setRememberOffered] = useState(true);
  const [method, setMethod] = useState<Method>("totp");
  const [code, setCode] = useState("");
  const [remember, setRemember] = useState(false);
  const [error, setError] = useState<string | undefined>(undefined);
  const [attempt, setAttempt] = useState(0);
  const [busy, setBusy] = useState(false);

  useDocumentTitle("Two-step sign-in");

  // The form is drawn at once; the pending sign-in is checked behind it.
  useEffect(() => {
    let live = true;

    auth.challenge().then(
      (next) => {
        if (!live) return;

        if (next.kind === "secondFactorRequired") {
          setMethods(next.challenge.methods);
          setRememberOffered(next.challenge.rememberDevice);
        } else if (next.kind !== "error") {
          follow(next);
        }
      },
      () => undefined,
    );

    return () => {
      live = false;
    };
  }, [follow]);

  const refuse = (message: string) => {
    setError(message);
    setAttempt((count) => count + 1);
    setCode("");
    setBusy(false);
    codeRef.current?.focus();
  };

  const answered = (next: AuthNext) => {
    if (next.kind === "error") {
      refuse(refusalMessage(next.fieldErrors, "code", "base"));

      return;
    }

    follow(next);
  };

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    if (busy) return;

    setBusy(true);
    auth
      .verify(code, rememberOffered && remember)
      .then(answered, (failure: Error) => refuse(failure.message));
  };

  const other: Method | null =
    method === "totp"
      ? methods.includes("recoveryCode")
        ? "recoveryCode"
        : null
      : methods.includes("totp")
        ? "totp"
        : null;

  const switchMethod = (next: Method) => {
    setMethod(next);
    setCode("");
    setError(undefined);
    codeRef.current?.focus();
  };

  const field = FIELDS[method];

  return (
    <AuthScreen labelledBy={titleId}>
      <AuthHeader
        titleId={titleId}
        title="Enter your code"
        lede="Open your authenticator app and enter the 6-digit code. Lost your phone? Use one of your backup codes instead."
      />

      <form className="auth-view-form" onSubmit={submit} aria-labelledby={titleId}>
        <div className="auth-view-code">
          <TextField
            ref={codeRef}
            label={field.label}
            className="auth-view-code-input"
            name="code"
            required
            autoFocus
            autoComplete={field.autoComplete}
            inputMode={field.inputMode}
            autoCapitalize="off"
            spellCheck={false}
            maxLength={20}
            placeholder={field.placeholder}
            value={code}
            onChange={(event) => setCode(event.target.value)}
            error={error}
            attempt={attempt}
          />
          {other === null ? null : (
            <Button
              variant="link"
              size="sm"
              className="auth-view-switch"
              onClick={() => switchMethod(other)}
            >
              {field.switchTo}
            </Button>
          )}
        </div>

        {rememberOffered ? (
          <Checkbox
            checked={remember}
            onCheckedChange={setRemember}
            label="Remember this device for 30 days"
          />
        ) : null}

        <Button
          type="submit"
          variant="primary"
          size="lg"
          className="auth-view-wide"
          loading={busy}
          loadingLabel="Checking…"
        >
          Sign in
        </Button>
      </form>
    </AuthScreen>
  );
}
