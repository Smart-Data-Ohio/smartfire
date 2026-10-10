import { Link, useParams } from "@tanstack/react-router";
import { useEffect, useId, useRef, useState } from "react";
import { type AuthNext, auth, inlineSignedOutBoot } from "../../sync/auth.ts";
import { Button, Spinner } from "../../ui/button.tsx";
import {
  AuthHeader,
  AuthScreen,
  refusalMessage,
  useFollowNext,
  useRestoredFromCache,
} from "./auth-parts.tsx";

type Landing =
  | { readonly status: "working" }
  | { readonly status: "restored" }
  | { readonly status: "refused"; readonly message: string }
  | { readonly status: "failed"; readonly message: string };

/** The route's page, started over for each link. */
export function TransferRoute() {
  const { transferId } = useParams({ from: "/session/transfers/$transferId" });

  return <TransferPage key={transferId} transferId={transferId} />;
}

/**
 * `/app/session/transfers/:id`: the sign-in link's landing (sessions/transfers/show.html). It
 * signs in by itself, as the retained page's form submits itself, then goes on to the app or the
 * second step. An expired or unknown link says so and offers the sign-in page, where the retained
 * handler answered an empty 400.
 */
export function TransferPage({ transferId }: { readonly transferId: string }) {
  const titleId = useId();
  const follow = useFollowNext();
  const [landing, setLanding] = useState<Landing>({ status: "working" });
  const [logoUrl] = useState(() => inlineSignedOutBoot()?.workspace.logoUrl ?? null);
  const sent = useRef<string | null>(null);

  const consume = (id: string) => {
    sent.current = id;
    setLanding({ status: "working" });
    auth.transfer(id).then(
      (next: AuthNext) => {
        if (next.kind === "error") {
          setLanding({ status: "refused", message: refusalMessage(next.fieldErrors, "base") });
        } else {
          follow(next);
        }
      },
      (error: Error) => setLanding({ status: "failed", message: error.message }),
    );
  };

  // Once per link: development's double effect would sign in twice.
  // biome-ignore lint/correctness/useExhaustiveDependencies: keyed on the link alone
  useEffect(() => {
    if (sent.current !== transferId) consume(transferId);
  }, [transferId]);

  // Back from the app restores the "Signing you in" state. The link is still good and each use
  // is another session, so it isn't sent again by itself (the retained page's form submits once,
  // on load); the page offers to continue instead.
  useRestoredFromCache(() =>
    setLanding((current) => (current.status === "working" ? { status: "restored" } : current)),
  );

  useEffect(() => {
    document.title = landing.status === "working" ? "Smartfire" : "Sign-in link";
  }, [landing.status]);

  if (landing.status === "working") {
    return (
      <AuthScreen labelledBy={titleId} busy>
        <AuthHeader titleId={titleId} title="Signing you in" logoUrl={logoUrl} />
        <p className="auth-view-wait" role="status">
          <Spinner />
          One moment…
        </p>
      </AuthScreen>
    );
  }

  if (landing.status === "restored") {
    return (
      <AuthScreen labelledBy={titleId}>
        <AuthHeader titleId={titleId} title="Sign-in link" logoUrl={logoUrl} />
        <div className="auth-view-stack">
          <Button
            variant="primary"
            size="lg"
            className="auth-view-wide"
            onClick={() => consume(transferId)}
          >
            Continue
          </Button>
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

  return (
    <AuthScreen labelledBy={titleId}>
      <AuthHeader titleId={titleId} title="Couldn't sign you in" logoUrl={logoUrl} />
      <div className="auth-view-stack">
        <p className="auth-view-alert" role="alert">
          {landing.message}
        </p>
        {landing.status === "failed" ? (
          <Button
            variant="primary"
            size="lg"
            className="auth-view-wide"
            onClick={() => consume(transferId)}
          >
            Try again
          </Button>
        ) : null}
        <Link
          to="/session/new"
          className="button auth-view-wide"
          data-variant={landing.status === "failed" ? "secondary" : "primary"}
          data-size="lg"
        >
          Go to sign in
        </Link>
      </div>
    </AuthScreen>
  );
}
