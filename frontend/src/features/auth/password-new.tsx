import { Link, useSearch } from "@tanstack/react-router";
import { type FormEvent, useId, useRef, useState } from "react";
import { auth, type PasswordResetNext, type SignedOutBootData } from "../../sync/auth.ts";
import { Button, Spinner } from "../../ui/button.tsx";
import { TextField } from "../../ui/text-field.tsx";
import {
  AuthFooter,
  AuthHeader,
  AuthScreen,
  refusalMessage,
  type SignedOutFlash,
  TranslateButton,
  useDocumentTitle,
  useFollowNext,
  useRestoredFromCache,
  useSignedOutBoot,
} from "./auth-parts.tsx";
import { EMAIL_TRANSLATIONS } from "./translations.ts";

const TITLE = "Reset your password";

/** What the reset pages say when this workspace can't email reset links. */
export const RESET_UNAVAILABLE =
  "This workspace can't email password reset links. Ask a workspace administrator to reset your password.";

/**
 * `/app/passwords/new`: "Forgot password?". Asks for the account's email address and says a link
 * is on its way, the same for every address, whether or not it has an account.
 */
export function NewPasswordPage() {
  const { load, retry, flash } = useSignedOutBoot();
  const titleId = useId();

  useDocumentTitle(TITLE);

  if (load.status === "ready") return <RequestForm boot={load.boot} flash={flash} />;

  return (
    <AuthScreen labelledBy={titleId} busy={load.status === "loading"} flash={flash}>
      <AuthHeader titleId={titleId} title={TITLE} />
      {load.status === "error" ? (
        <div className="auth-view-stack" role="alert">
          <p className="auth-view-lede">{load.message}</p>
          <Button variant="primary" size="lg" className="auth-view-wide" onClick={retry}>
            Try again
          </Button>
        </div>
      ) : (
        <p className="auth-view-wait" role="status">
          <Spinner />
          One moment…
        </p>
      )}
    </AuthScreen>
  );
}

function RequestForm({
  boot,
  flash,
}: {
  readonly boot: SignedOutBootData;
  readonly flash: SignedOutFlash | null;
}) {
  const titleId = useId();
  const follow = useFollowNext();
  const emailRef = useRef<HTMLInputElement | null>(null);
  // The sign-in page passes the address typed there.
  const filled = useSearch({ strict: false, select: (search) => search.email_address });
  const [email, setEmail] = useState(filled ?? "");
  const [sentTo, setSentTo] = useState<string | null>(null);
  const [error, setError] = useState<string | undefined>(undefined);
  const [attempt, setAttempt] = useState(0);
  const [busy, setBusy] = useState(false);

  useRestoredFromCache(() => setBusy(false));

  const { workspace } = boot;

  const signIn = (
    <Link
      to="/session/new"
      search={email.trim() === "" ? {} : { email_address: email.trim() }}
      className="button auth-view-wide"
      data-variant="secondary"
      data-size="lg"
    >
      Back to sign in
    </Link>
  );

  if (!boot.signInMethods.passwordReset) {
    return (
      <AuthScreen labelledBy={titleId} below={<AuthFooter boot={boot} />} flash={flash}>
        <AuthHeader titleId={titleId} title={TITLE} logoUrl={workspace.logoUrl} />
        <div className="auth-view-stack">
          <p className="auth-view-lede">{RESET_UNAVAILABLE}</p>
          {signIn}
        </div>
      </AuthScreen>
    );
  }

  if (sentTo !== null) {
    return (
      <AuthScreen labelledBy={titleId} below={<AuthFooter boot={boot} />} flash={flash}>
        <AuthHeader titleId={titleId} title="Check your email" logoUrl={workspace.logoUrl} />
        <div className="auth-view-stack" role="status">
          <p className="auth-view-lede">
            If {sentTo} belongs to an account that signs in with a password, we've emailed it a link
            to choose a new one. The link expires in 15 minutes.
          </p>
          {signIn}
          <p className="auth-view-note">
            Didn't get it? Check your spam folder, or{" "}
            <Button variant="link" size="sm" onClick={() => setSentTo(null)}>
              send another link
            </Button>
            .
          </p>
        </div>
      </AuthScreen>
    );
  }

  const refuse = (message: string) => {
    setError(message);
    setAttempt((count) => count + 1);
    setBusy(false);
    emailRef.current?.focus();
  };

  const answered = (next: PasswordResetNext) => {
    switch (next.kind) {
      case "requested":
      case "ready":
        setBusy(false);
        setError(undefined);
        setSentTo(email.trim());

        return;
      case "error":
        refuse(refusalMessage(next.fieldErrors, "base", "emailAddress"));

        return;
      default:
        follow(next);
    }
  };

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    if (busy) return;

    setBusy(true);
    auth.requestPasswordReset(email).then(answered, (failure: Error) => refuse(failure.message));
  };

  return (
    <AuthScreen labelledBy={titleId} below={<AuthFooter boot={boot} />} flash={flash}>
      <AuthHeader
        titleId={titleId}
        title={TITLE}
        logoUrl={workspace.logoUrl}
        lede="Enter your email address and we'll send you a link to choose a new password."
      />

      <form className="auth-view-form" onSubmit={submit} aria-labelledby={titleId}>
        <TextField
          ref={emailRef}
          label="Email address"
          labelAccessory={
            <TranslateButton field="Email address" translations={EMAIL_TRANSLATIONS} />
          }
          type="email"
          name="email_address"
          required
          autoFocus
          autoComplete="username"
          placeholder="you@example.com"
          value={email}
          onChange={(event) => setEmail(event.target.value)}
          error={error}
          attempt={attempt}
        />
        <Button
          type="submit"
          variant="primary"
          size="lg"
          className="auth-view-wide"
          loading={busy}
          loadingLabel="Sending…"
        >
          Email me a link
        </Button>
      </form>

      {signIn}
    </AuthScreen>
  );
}
