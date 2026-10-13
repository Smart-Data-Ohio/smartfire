import { useSearch } from "@tanstack/react-router";
import { useId, useState } from "react";
import { inlineSignedOutBoot } from "../../sync/auth.ts";
import { AuthHeader, AuthScreen, useDocumentTitle } from "../auth/auth-parts.tsx";
import { oauthErrorText } from "./oauth-format.ts";
import type { OAuthErrorSearch } from "./oauth-search.ts";
import "../auth/auth.css";
import "./oauth.css";

/** The route's page, reading its query. */
export function OAuthErrorRoute() {
  return <OAuthErrorPage search={useSearch({ from: "/oauth/error" })} />;
}

/**
 * `/app/oauth/error?error=`: an app's request to connect that Smartfire couldn't take (it can't
 * tell who the app is, or where to send the answer, so it can't hand the error back). Signed in or
 * not, it says what went wrong in plain words and offers the way back.
 */
export function OAuthErrorPage({ search }: { readonly search: OAuthErrorSearch }) {
  const titleId = useId();
  const [logoUrl] = useState(() => inlineSignedOutBoot()?.workspace.logoUrl ?? null);
  const { title, message } = oauthErrorText(search.error ?? null);

  useDocumentTitle(title);

  return (
    <AuthScreen labelledBy={titleId}>
      <AuthHeader titleId={titleId} title={title} logoUrl={logoUrl} />
      <div className="auth-view-stack">
        <p className="auth-view-alert" role="alert">
          {message}
        </p>
        {search.error_description === undefined ? null : (
          <p className="oauth-error-detail text-muted">
            The app said: <q>{search.error_description}</q>
          </p>
        )}
        {search.error === undefined ? null : (
          <p className="oauth-error-detail text-faint">
            Error code: <code>{search.error}</code>
          </p>
        )}
        <a
          className="button auth-view-wide"
          data-variant="primary"
          data-size="lg"
          href={import.meta.env.BASE_URL}
        >
          Back to Smartfire
        </a>
      </div>
    </AuthScreen>
  );
}
