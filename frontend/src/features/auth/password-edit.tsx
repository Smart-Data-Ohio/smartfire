import { Link, useParams } from "@tanstack/react-router";
import { type FormEvent, useEffect, useId, useRef, useState } from "react";
import { auth, inlineSignedOutBoot, type PasswordResetNext } from "../../sync/auth.ts";
import { Button, Spinner } from "../../ui/button.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { pageExit } from "./auth-navigation.ts";
import {
  AuthHeader,
  AuthScreen,
  GENERIC_REFUSAL,
  refusalMessage,
  useDocumentTitle,
  useFollowNext,
  useInlineFlash,
  useRestoredFromCache,
} from "./auth-parts.tsx";

const TITLE = "Reset your password";

type Landing =
  | { readonly status: "checking" }
  | { readonly status: "ready" }
  | { readonly status: "refused"; readonly message: string }
  | { readonly status: "failed"; readonly message: string };

/** The route's page, started over for each link. */
export function EditPasswordRoute() {
  const { token } = useParams({ from: "/passwords/$token" });

  return <EditPasswordPage key={token} token={token} />;
}

/**
 * `/app/passwords/:token`: the reset link's landing. It checks the link first, so an expired one
 * says so before a password is typed, then sets the new password and goes to sign in.
 */
export function EditPasswordPage({ token }: { readonly token: string }) {
  const titleId = useId();
  const flash = useInlineFlash();
  const [logoUrl] = useState(() => inlineSignedOutBoot()?.workspace.logoUrl ?? null);
  const [landing, setLanding] = useState<Landing>({ status: "checking" });

  useDocumentTitle(TITLE);

  const check = () => {
    setLanding({ status: "checking" });
    auth.passwordReset(token).then(
      (next) =>
        setLanding(
          next.kind === "ready"
            ? { status: "ready" }
            : { status: "refused", message: refusal(next) },
        ),
      (error: Error) => setLanding({ status: "failed", message: error.message }),
    );
  };

  useEffect(check, [token]);

  if (landing.status === "checking") {
    return (
      <AuthScreen labelledBy={titleId} busy flash={flash}>
        <AuthHeader titleId={titleId} title={TITLE} logoUrl={logoUrl} />
        <p className="auth-view-wait" role="status">
          <Spinner />
          One moment…
        </p>
      </AuthScreen>
    );
  }

  if (landing.status === "ready") {
    return (
      <NewPasswordForm
        token={token}
        logoUrl={logoUrl}
        flash={flash}
        refused={(message) => setLanding({ status: "refused", message })}
      />
    );
  }

  return (
    <AuthScreen labelledBy={titleId} flash={flash}>
      <AuthHeader titleId={titleId} title={TITLE} logoUrl={logoUrl} />
      <div className="auth-view-stack">
        <p className="auth-view-alert" role="alert">
          {landing.message}
        </p>
        {landing.status === "failed" ? (
          <Button variant="primary" size="lg" className="auth-view-wide" onClick={check}>
            Try again
          </Button>
        ) : (
          <Link
            to="/passwords/new"
            className="button auth-view-wide"
            data-variant="primary"
            data-size="lg"
          >
            Get a new link
          </Link>
        )}
        <Link
          to="/session/new"
          className="button auth-view-wide"
          data-variant="secondary"
          data-size="lg"
        >
          Go to sign in
        </Link>
      </div>
    </AuthScreen>
  );
}

function NewPasswordForm({
  token,
  logoUrl,
  flash,
  refused,
}: {
  readonly token: string;
  readonly logoUrl: string | null;
  readonly flash: ReturnType<typeof useInlineFlash>;
  readonly refused: (message: string) => void;
}) {
  const titleId = useId();
  const follow = useFollowNext();
  const passwordRef = useRef<HTMLInputElement | null>(null);
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | undefined>(undefined);
  const [attempt, setAttempt] = useState(0);
  const [busy, setBusy] = useState(false);

  useRestoredFromCache(() => setBusy(false));

  const refuse = (message: string) => {
    setError(message);
    setAttempt((count) => count + 1);
    setBusy(false);
    passwordRef.current?.focus();
  };

  const answered = (next: PasswordResetNext) => {
    switch (next.kind) {
      case "error": {
        const problem = next.fieldErrors.password?.[0];

        if (problem === undefined) {
          refused(refusalMessage(next.fieldErrors, "base"));
        } else {
          refuse(problem);
        }

        return;
      }

      case "navigate":
        // A full load, so the sign-in page shows the notice the server left for it. Busy stays on
        // while the page leaves.
        pageExit.assign(next.location);

        return;
      case "ready":
      case "requested":
        refuse(GENERIC_REFUSAL);

        return;
      default:
        follow(next);
    }
  };

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    if (busy) return;

    setBusy(true);
    auth.resetPassword(token, password).then(answered, (failure: Error) => refuse(failure.message));
  };

  return (
    <AuthScreen labelledBy={titleId} flash={flash}>
      <AuthHeader
        titleId={titleId}
        title="Choose a new password"
        logoUrl={logoUrl}
        lede="You'll sign in with it next. Every device signed in to your account is signed out."
      />

      <form className="auth-view-form" onSubmit={submit} aria-labelledby={titleId}>
        <TextField
          ref={passwordRef}
          label="New password"
          type="password"
          name="password"
          required
          autoFocus
          autoComplete="new-password"
          maxLength={72}
          value={password}
          onChange={(event) => setPassword(event.target.value)}
          error={error}
          attempt={attempt}
        />
        <Button
          type="submit"
          variant="primary"
          size="lg"
          className="auth-view-wide"
          loading={busy}
          loadingLabel="Saving…"
        >
          Reset password
        </Button>
      </form>
    </AuthScreen>
  );
}

/** Why the link can't be used: the server's reason, else a general one. */
function refusal(next: PasswordResetNext): string {
  return next.kind === "error" ? refusalMessage(next.fieldErrors, "base") : GENERIC_REFUSAL;
}
