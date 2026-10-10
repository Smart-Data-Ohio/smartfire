import { useNavigate } from "@tanstack/react-router";
import { type ReactNode, useCallback, useEffect, useEffectEvent, useState } from "react";
import {
  type AuthNext,
  auth,
  inlineSignedOutBoot,
  type SignedOutBootData,
} from "../../sync/auth.ts";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Popover } from "../../ui/popover.tsx";
import { pageExit, signedOutPageFor } from "./auth-navigation.ts";
import type { Translation } from "./translations.ts";
import "./auth.css";

/** Said when a refusal names no reason (an access gate's empty answer). */
export const GENERIC_REFUSAL = "Couldn't sign you in. Try again.";

/** The first message the server put on any of `keys`, else on any field. */
export function refusalMessage(
  fieldErrors: Readonly<Record<string, readonly string[]>>,
  ...keys: readonly string[]
): string {
  for (const key of keys) {
    const message = fieldErrors[key]?.[0];

    if (message !== undefined) return message;
  }

  return Object.values(fieldErrors).flat()[0] ?? GENERIC_REFUSAL;
}

/** Sets the tab's title while a signed-out page is up, as the retained pages' `<title>`. */
export function useDocumentTitle(title: string): void {
  useEffect(() => {
    document.title = title;
  }, [title]);
}

/**
 * Follows a next action that isn't a refusal: signing in (and Google, and anywhere the SPA
 * doesn't draw) is a full page load; a challenge or the sign-in page stays in the SPA. Answers
 * whether the page is leaving, so a form keeps its busy state until it goes.
 */
export function useFollowNext(): (next: Exclude<AuthNext, { readonly kind: "error" }>) => void {
  const navigate = useNavigate();

  return useCallback(
    (next) => {
      if (next.kind === "secondFactorRequired") {
        void navigate({ to: "/two_factor/challenge", replace: true });

        return;
      }

      const page = next.kind === "navigate" ? signedOutPageFor(next.location) : null;

      if (page === null) {
        pageExit.assign(next.location);
      } else {
        void navigate({ to: page, replace: true });
      }
    },
    [navigate],
  );
}

type BootLoad =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly boot: SignedOutBootData };

/** The boot as it loads, and a way to ask again after a failure. */
interface SignedOutBootState {
  readonly load: BootLoad;
  readonly retry: () => void;
}

/**
 * The public boot: inline from the Rust shell when it rendered this page signed out, else fetched
 * from `GET /api/v1/session/boot` (the Vite dev page, or a signed-in visitor here).
 */
export function useSignedOutBoot(): SignedOutBootState {
  const [load, setLoad] = useState<BootLoad>(() => {
    const inline = inlineSignedOutBoot();

    return inline === null ? { status: "loading" } : { status: "ready", boot: inline };
  });

  const fetchBoot = useCallback(() => {
    auth.boot().then(
      (boot) => setLoad({ status: "ready", boot }),
      (error: Error) => setLoad({ status: "error", message: error.message }),
    );
  }, []);

  // biome-ignore lint/correctness/useExhaustiveDependencies: fetched once, unless it came inline
  useEffect(() => {
    if (load.status === "loading") fetchBoot();
  }, []);

  const retry = useCallback(() => {
    setLoad({ status: "loading" });
    fetchBoot();
  }, [fetchBoot]);

  return { load, retry };
}

/** The page: one centred card on the app background, and whatever goes under it. */
export function AuthScreen({
  children,
  below,
  labelledBy,
  busy,
}: {
  readonly children: ReactNode;
  readonly below?: ReactNode;
  readonly labelledBy: string;
  readonly busy?: boolean;
}) {
  return (
    <div className="auth-view">
      <main className="auth-view-card" aria-labelledby={labelledBy} aria-busy={busy || undefined}>
        {children}
      </main>
      {below}
    </div>
  );
}

/**
 * Calls `restored` when the browser brings the page back from its back/forward cache (Back after
 * leaving for Google or the app), so a form left busy by the departure can be used again, as the
 * retained auth.js clears its submitting forms on `pageshow`.
 */
export function useRestoredFromCache(restored: () => void): void {
  const onRestore = useEffectEvent(restored);

  useEffect(() => {
    const show = (event: PageTransitionEvent) => {
      if (event.persisted) onRestore();
    };

    window.addEventListener("pageshow", show);

    return () => window.removeEventListener("pageshow", show);
  }, []);
}

/** The card's header: the workspace's logo when there is one, a title, a line or two under it. */
export function AuthHeader({
  titleId,
  title,
  logoUrl,
  lede,
  children,
}: {
  readonly titleId: string;
  readonly title: string;
  readonly logoUrl?: string | null;
  readonly lede?: string;
  readonly children?: ReactNode;
}) {
  return (
    <header className="auth-view-header">
      {logoUrl === null || logoUrl === undefined ? null : (
        <img className="auth-view-logo" src={logoUrl} alt="" width={48} height={48} />
      )}
      <h1 id={titleId} className="auth-view-title">
        {title}
      </h1>
      {lede === undefined ? null : <p className="auth-view-lede">{lede}</p>}
      {children}
    </header>
  );
}

/** The globe beside a field's label, opening that label in other languages. */
export function TranslateButton({
  field,
  translations,
}: {
  readonly field: string;
  readonly translations: readonly Translation[];
}) {
  const label = `Translate ${field.toLowerCase()}`;

  return (
    <Popover
      label={label}
      placement="bottom-end"
      trigger={(props) => <IconButton {...props} icon="globe" label={label} size="sm" />}
    >
      <dl className="auth-view-translations">
        {translations.map((entry) => (
          <div key={entry.flag} className="auth-view-translation">
            <dt>
              <span role="img" aria-label={FLAG_LANGUAGES.get(entry.flag) ?? entry.flag}>
                {entry.flag}
              </span>
            </dt>
            <dd>{entry.text}</dd>
          </div>
        ))}
      </dl>
    </Popover>
  );
}

/** What each flag in the translation lists stands for, for screen readers. */
const FLAG_LANGUAGES = new Map([
  ["🇺🇸", "English"],
  ["🇪🇸", "Spanish"],
  ["🇫🇷", "French"],
  ["🇮🇳", "Hindi"],
  ["🇩🇪", "German"],
  ["🇧🇷", "Portuguese"],
  ["🇯🇵", "Japanese"],
]);

/**
 * Under the sign-in card: the public pages (in a new tab, as the retained page opens them), then
 * the first administrator to ask for help and the version.
 */
export function AuthFooter({ boot }: { readonly boot: SignedOutBootData }) {
  const contact = boot.helpContact;

  return (
    <>
      <nav className="auth-view-links" aria-label="About this workspace">
        <a href="/about" target="_blank" rel="noopener">
          About
        </a>
        <a href="/privacy" target="_blank" rel="noopener">
          Privacy Policy
        </a>
        <a href="/terms" target="_blank" rel="noopener">
          Terms of Service
        </a>
      </nav>
      {contact === null ? null : (
        <div className="auth-view-help">
          <a href={`mailto:${contact.emailAddress}`} title={`Email ${contact.name}`}>
            <Icon name="life-buoy" size={14} />
            <span>{contact.emailAddress}</span>
          </a>
          <span>Smartfire™ version {boot.version}</span>
        </div>
      )}
    </>
  );
}

/** Google's four-colour "G", as the retained button draws it. */
export function GoogleMark() {
  return (
    <svg
      className="auth-view-google-mark"
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
