import { type ReactNode, useCallback, useEffect, useState } from "react";
import type { PublicPageName } from "../../gen/PublicPageName.ts";
import { auth, type PublicPageData } from "../../sync/auth.ts";
import { Button, Spinner } from "../../ui/button.tsx";
import { useDocumentTitle } from "./auth-parts.tsx";
import "./auth.css";
import "./public-page.css";

type PageLoad =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly page: PublicPageData };

/** The page's contract, as it loads, and a way to ask again after a failure. */
interface PublicPageState {
  readonly load: PageLoad;
  readonly retry: () => void;
}

function usePublicPage(name: PublicPageName): PublicPageState {
  const [load, setLoad] = useState<PageLoad>({ status: "loading" });

  const fetchPage = useCallback(() => {
    let current = true;

    setLoad({ status: "loading" });
    auth.publicPage(name).then(
      (page) => {
        if (current) setLoad({ status: "ready", page });
      },
      (error: Error) => {
        if (current) setLoad({ status: "error", message: error.message });
      },
    );

    return () => {
      current = false;
    };
  }, [name]);

  useEffect(fetchPage, [fetchPage]);

  return { load, retry: fetchPage };
}

/** Sets the page's meta description while it is up, as the retained page's head carries it. */
function useMetaDescription(description: string | null): void {
  useEffect(() => {
    if (description === null) return;

    const meta = document.createElement("meta");

    meta.name = "description";
    meta.content = description;
    document.head.append(meta);

    return () => meta.remove();
  }, [description]);
}

/** A public page's link: in a new tab, as the retained layout opens each of them. */
function PageLink({ to, children }: { readonly to: string; readonly children: ReactNode }) {
  return (
    <a href={to} target="_blank" rel="noopener">
      {children}
    </a>
  );
}

/** The article once it has loaded; until then, a wait or a failure to try again from. */
function PageBody({ load, retry }: PublicPageState) {
  if (load.status === "ready") {
    return (
      <article
        className="public-view-prose"
        // biome-ignore lint/security/noDangerouslySetInnerHtml: the retained page's own Askama render (escaped LEGAL_* values, fixed text), the HTML /about, /privacy and /terms serve
        dangerouslySetInnerHTML={{ __html: load.page.html }}
      />
    );
  }

  if (load.status === "error") {
    return (
      <div className="public-view-state" role="alert">
        <p>{load.message}</p>
        <Button variant="primary" onClick={retry}>
          Try again
        </Button>
      </div>
    );
  }

  return (
    <p className="auth-view-wait public-view-state" role="status">
      <Spinner />
      One moment…
    </p>
  );
}

/**
 * `/app/about`, `/app/privacy` and `/app/terms`: the retained public pages
 * (crates/retained_pages/templates/public_pages, in layouts/public.html) for anyone, signed in or
 * out. The article is the retained template's own render, from `GET /api/v1/public_pages/:page`,
 * with its links on these pages; the header and footer are the retained layout's. Nothing else
 * in the app links here but the sign-in page, as nothing in the retained app links the public
 * pages but sign-in and the pages themselves.
 */
export function PublicPageView({ name }: { readonly name: PublicPageName }) {
  const { load, retry } = usePublicPage(name);

  useDocumentTitle(load.status === "ready" ? load.page.title : "Smartfire");
  useMetaDescription(load.status === "ready" ? load.page.description : null);

  return (
    <div className="public-view">
      <a className="public-view-skip" href="#public-main">
        Skip to main content
      </a>
      <header className="public-view-header">
        <div className="public-view-wrap public-view-header-inner">
          <a className="public-view-brand" href="/app/about" target="_blank" rel="noopener">
            Smartfire
          </a>
          <nav className="public-view-nav" aria-label="Public pages">
            <PageLink to="/app/about">About</PageLink>
            <PageLink to="/app/privacy">Privacy</PageLink>
            <PageLink to="/app/terms">Terms</PageLink>
            <a
              href="/app/session/new"
              className="button public-view-signin"
              data-variant="primary"
              data-size="sm"
            >
              Sign in
            </a>
          </nav>
        </div>
      </header>
      <main
        id="public-main"
        tabIndex={-1}
        className="public-view-wrap public-view-main"
        aria-busy={load.status === "loading" || undefined}
      >
        <PageBody load={load} retry={retry} />
      </main>
      <footer className="public-view-footer">
        <div className="public-view-wrap">
          <nav className="public-view-nav" aria-label="Footer">
            <PageLink to="/app/about">About</PageLink>
            <PageLink to="/app/privacy">Privacy Policy</PageLink>
            <PageLink to="/app/terms">Terms of Service</PageLink>
            <a href="/app/session/new">Sign in</a>
          </nav>
          <p>
            Smartfire is self-hosted software. Each workspace is run by its own hosting
            organization, which controls that workspace's data.
          </p>
        </div>
      </footer>
    </div>
  );
}

export function AboutPage() {
  return <PublicPageView name="about" />;
}

export function PrivacyPage() {
  return <PublicPageView name="privacy" />;
}

export function TermsPage() {
  return <PublicPageView name="terms" />;
}
