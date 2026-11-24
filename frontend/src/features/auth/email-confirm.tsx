import { useParams } from "@tanstack/react-router";
import { useEffect, useId, useState } from "react";
import { auth, type EmailConfirmationNext, inlineSignedOutBoot } from "../../sync/auth.ts";
import { Button, Spinner } from "../../ui/button.tsx";
import { pageExit } from "./auth-navigation.ts";
import {
  AuthHeader,
  AuthScreen,
  refusalMessage,
  useDocumentTitle,
  useInlineFlash,
  useRestoredFromCache,
} from "./auth-parts.tsx";

const TITLE = "Confirm your email address";

const COULD_NOT_CONFIRM = "Couldn't confirm this email address. Try again.";

type Landing =
  | { readonly status: "checking" }
  | { readonly status: "ready"; readonly emailAddress: string; readonly requiresSignIn: boolean }
  | { readonly status: "refused"; readonly message: string }
  | { readonly status: "failed"; readonly message: string };

/** The route's page, started over for each link. */
export function ConfirmEmailRoute() {
  const { token } = useParams({ from: "/email_addresses/confirm/$token" });

  return <ConfirmEmailPage key={token} token={token} />;
}

/** What a confirmation endpoint's refusal says: the link's problem or the address's. */
function refusal(next: EmailConfirmationNext): string {
  return "kind" in next && next.kind === "error"
    ? refusalMessage(next.fieldErrors, "emailAddress", "base")
    : COULD_NOT_CONFIRM;
}

/**
 * `/app/email_addresses/confirm/:token`: the emailed link's landing. It shows the address and waits
 * for Confirm (mail scanners open links, so loading the page links nothing), then follows the
 * server's next page with a full load so its notice shows.
 */
export function ConfirmEmailPage({ token }: { readonly token: string }) {
  const titleId = useId();
  const flash = useInlineFlash();
  const [logoUrl] = useState(() => inlineSignedOutBoot()?.workspace.logoUrl ?? null);
  const [landing, setLanding] = useState<Landing>({ status: "checking" });
  const [busy, setBusy] = useState(false);

  useDocumentTitle(TITLE);
  useRestoredFromCache(() => setBusy(false));

  const check = () => {
    setLanding({ status: "checking" });
    auth.emailConfirmation(token).then(
      (next) =>
        setLanding(
          "emailAddress" in next
            ? { status: "ready", ...next }
            : { status: "refused", message: refusal(next) },
        ),
      (error: Error) => setLanding({ status: "failed", message: error.message }),
    );
  };

  useEffect(check, [token]);

  const confirm = () => {
    if (busy) return;

    setBusy(true);
    auth.confirmEmailAddress(token).then(
      (next) => {
        if ("kind" in next && next.kind === "navigate") {
          // Busy stays on while the page leaves.
          pageExit.assign(next.location);

          return;
        }

        setBusy(false);
        setLanding({ status: "refused", message: refusal(next) });
      },
      (error: Error) => {
        setBusy(false);
        setLanding({ status: "failed", message: error.message });
      },
    );
  };

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
      <AuthScreen labelledBy={titleId} flash={flash}>
        <AuthHeader
          titleId={titleId}
          title={TITLE}
          logoUrl={logoUrl}
          lede="Link this address to your account. You can use it to sign in and reset your password."
        />
        <div className="auth-view-stack">
          <p className="auth-view-address">{landing.emailAddress}</p>
          <Button
            variant="primary"
            size="lg"
            className="auth-view-wide"
            loading={busy}
            loadingLabel="Confirming…"
            onClick={confirm}
          >
            Confirm
          </Button>
          {landing.requiresSignIn ? (
            <p className="auth-view-note">You'll go to the sign-in page after confirming.</p>
          ) : null}
        </div>
      </AuthScreen>
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
        ) : null}
        {/* A full load: the settings page is the signed-in app, outside this signed-out shell. */}
        <a
          href="/app/settings/profile"
          className="button auth-view-wide"
          data-variant="secondary"
          data-size="lg"
        >
          Go to settings
        </a>
      </div>
    </AuthScreen>
  );
}
