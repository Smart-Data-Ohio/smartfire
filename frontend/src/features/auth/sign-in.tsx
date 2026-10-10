import { type FormEvent, useEffect, useId, useRef, useState } from "react";
import { type AuthNext, auth, type SignedOutBootData } from "../../sync/auth.ts";
import { Button, Spinner } from "../../ui/button.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { FIRST_RUN_PATH, pageExit } from "./auth-navigation.ts";
import {
  AuthFooter,
  AuthHeader,
  AuthScreen,
  GoogleMark,
  refusalMessage,
  TranslateButton,
  useDocumentTitle,
  useFollowNext,
  useRestoredFromCache,
  useSignedOutBoot,
} from "./auth-parts.tsx";
import { EMAIL_TRANSLATIONS, PASSWORD_TRANSLATIONS } from "./translations.ts";

/**
 * `/app/session/new`: the retained sign-in page (crates/retained_pages/templates/sessions/new.html)
 * in the SPA. Email and password, Google when it is configured, the workspace's name, logo and
 * description, then the public pages and whom to ask for help. A fresh install goes on to first
 * run, as the retained page sends it.
 */
export function SignInPage() {
  const { load, retry } = useSignedOutBoot();
  const titleId = useId();
  const firstRun = load.status === "ready" && load.boot.firstRunPending;

  useDocumentTitle("Sign in");

  useEffect(() => {
    if (firstRun) pageExit.replace(FIRST_RUN_PATH);
  }, [firstRun]);

  if (load.status === "ready" && !firstRun) {
    return <SignInForm boot={load.boot} />;
  }

  return (
    <AuthScreen labelledBy={titleId} busy={load.status === "loading" || firstRun}>
      <AuthHeader titleId={titleId} title="Sign in" />
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

type Busy = "password" | "google" | null;

function SignInForm({ boot }: { readonly boot: SignedOutBootData }) {
  const titleId = useId();
  const follow = useFollowNext();
  const passwordRef = useRef<HTMLInputElement | null>(null);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | undefined>(undefined);
  const [attempt, setAttempt] = useState(0);
  const [busy, setBusy] = useState<Busy>(null);

  useRestoredFromCache(() => setBusy(null));

  const refuse = (message: string) => {
    setError(message);
    setAttempt((count) => count + 1);
    setPassword("");
    setBusy(null);
    passwordRef.current?.focus();
  };

  const answered = (next: AuthNext) => {
    if (next.kind === "error") {
      refuse(refusalMessage(next.fieldErrors, "base", "emailAddress", "password"));

      return;
    }

    // Busy stays on while the page leaves, so a second press can't send it again; a page restored
    // from the back/forward cache clears it.
    follow(next);
  };

  const failed = (failure: Error) => refuse(failure.message);

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    if (busy !== null) return;

    setBusy("password");
    auth.signIn(email, password).then(answered, failed);
  };

  const google = () => {
    if (busy !== null) return;

    setBusy("google");
    auth.google().then(answered, failed);
  };

  const { workspace, signInMethods } = boot;

  return (
    <AuthScreen labelledBy={titleId} below={<AuthFooter boot={boot} />}>
      <AuthHeader
        titleId={titleId}
        title={workspace.name ?? "Sign in"}
        logoUrl={workspace.logoUrl}
        lede="Sign in to your workspace"
      >
        {workspace.description === "" ? null : (
          <p className="auth-view-description">{workspace.description}</p>
        )}
      </AuthHeader>

      <form className="auth-view-form" onSubmit={submit} aria-labelledby={titleId}>
        <TextField
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
        />
        <TextField
          ref={passwordRef}
          label="Password"
          labelAccessory={<TranslateButton field="Password" translations={PASSWORD_TRANSLATIONS} />}
          type="password"
          name="password"
          required
          autoComplete="current-password"
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
          loading={busy === "password"}
          loadingLabel="Signing in…"
        >
          Sign in
        </Button>
      </form>

      {signInMethods.google ? (
        <div className="auth-view-stack">
          <p className="auth-view-divider">or</p>
          <Button
            variant="secondary"
            size="lg"
            className="auth-view-wide auth-view-google"
            loading={busy === "google"}
            onClick={google}
          >
            <GoogleMark />
            Sign in with Google
          </Button>
          <p className="auth-view-note">
            Google sign-in for {domainSentence(signInMethods.googleDomains)} accounts.
            <br />
            Other email addresses can sign in with email and password.
          </p>
        </div>
      ) : null}
    </AuthScreen>
  );
}

/** `@a.com`, `@a.com and @b.com`, `@a.com, @b.com, and @c.com`, as the retained note words it. */
export function domainSentence(domains: readonly string[]): string {
  const named = domains.map((domain) => `@${domain}`);

  if (named.length <= 2) return named.join(" and ");

  return `${named.slice(0, -1).join(", ")}, and ${named.at(-1)}`;
}
