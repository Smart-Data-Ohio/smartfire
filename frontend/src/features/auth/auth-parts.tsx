import { useNavigate } from "@tanstack/react-router";
import { type ReactNode, useCallback, useEffect, useEffectEvent, useRef, useState } from "react";
import { useKeyboardInset } from "../../lib/keyboard-inset.ts";
import {
  type AuthNext,
  auth,
  inlineSignedOutBoot,
  type SignedOutBootData,
} from "../../sync/auth.ts";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Popover } from "../../ui/popover.tsx";
import { pageExit, signedOutPageFor, signInSearchFor } from "./auth-navigation.ts";
import { InviteSplash } from "./invite-splash.tsx";
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
 * doesn't draw) is a full page load; a challenge or the sign-in page stays in the SPA.
 *
 * Only a page still on screen follows its answer. One the visitor has left (Sign in pressed while
 * a join was sending, say) drops it, so a late answer can't pull them off the page they went to.
 */
export function useFollowNext(): (next: Exclude<AuthNext, { readonly kind: "error" }>) => void {
  const navigate = useNavigate();
  const mounted = useRef(false);

  useEffect(() => {
    mounted.current = true;

    return () => {
      mounted.current = false;
    };
  }, []);

  return useCallback(
    (next) => {
      if (!mounted.current) return;

      if (next.kind === "secondFactorRequired") {
        void navigate({ to: "/two_factor/challenge", replace: true });

        return;
      }

      const page = next.kind === "navigate" ? signedOutPageFor(next.location) : null;

      if (page === null) {
        pageExit.assign(next.location);
      } else if (page === "/session/new" && next.kind === "navigate") {
        // An address that already has an account goes to sign-in with it filled in.
        void navigate({ to: page, search: signInSearchFor(next.location), replace: true });
      } else {
        void navigate({ to: page, replace: true });
      }
    },
    [navigate],
  );
}

/** The notice or alert a server redirect left for this page load (a refused Google sign-in, say). */
export type SignedOutFlash = NonNullable<SignedOutBootData["flash"]>;

/**
 * Inlined boots whose flash a page has shown. The flash belongs to the page load, so a signed-out
 * page opened after it in the SPA (sign-in, then the challenge) doesn't show it again.
 */
const flashShown = new WeakSet<Element>();

/** The flash the Rust shell inlined for this page load, for the first signed-out page drawn. */
export function useInlineFlash(): SignedOutFlash | null {
  const [flash] = useState(() => {
    const element = document.getElementById("boot");

    if (element === null || flashShown.has(element)) return null;

    return inlineSignedOutBoot()?.flash ?? null;
  });

  useEffect(() => {
    const element = document.getElementById("boot");

    if (element !== null) flashShown.add(element);
  }, []);

  return flash;
}

type BootLoad =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly boot: SignedOutBootData };

/** The boot as it loads, a way to ask again after a failure, and the page load's flash. */
interface SignedOutBootState {
  readonly load: BootLoad;
  readonly retry: () => void;
  readonly flash: SignedOutFlash | null;
}

/**
 * The public boot: inline from the Rust shell when it rendered this page signed out, else fetched
 * from `GET /api/v1/session/boot` (the Vite dev page, or a signed-in visitor here).
 */
export function useSignedOutBoot(): SignedOutBootState {
  const inlineFlash = useInlineFlash();
  const [fetchedFlash, setFetchedFlash] = useState<SignedOutFlash | null>(null);

  const [load, setLoad] = useState<BootLoad>(() => {
    const inline = inlineSignedOutBoot();

    return inline === null ? { status: "loading" } : { status: "ready", boot: inline };
  });

  const fetchBoot = useCallback(() => {
    auth.boot().then(
      (boot) => {
        setLoad({ status: "ready", boot });
        // The server hands the flash to one boot; a second fetch (development's double effect)
        // answers none, and mustn't clear it.
        setFetchedFlash((shown) => shown ?? boot.flash);
      },
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

  return { load, retry, flash: inlineFlash ?? fetchedFlash };
}

/** The page: one centred card on the app background, and whatever goes under it. */
export function AuthScreen({
  children,
  below,
  labelledBy,
  busy,
  flash,
  splashUrl = null,
}: {
  readonly splashUrl?: string | null;
  readonly children: ReactNode;
  readonly below?: ReactNode;
  readonly labelledBy: string;
  readonly busy?: boolean;
  readonly flash?: SignedOutFlash | null;
}) {
  // The view fills the visible viewport, which an overlaid (iOS) keyboard shrinks while viewport
  // units hold, so it follows the keyboard as the shell does.
  useKeyboardInset();

  return (
    <div
      className="invite-splash-surface auth-splash-surface"
      data-splash={splashUrl !== null || undefined}
    >
      <InviteSplash url={splashUrl} />
      <div className="auth-view">
        {flash === null || flash === undefined ? null : <AuthFlash flash={flash} />}
        <main className="auth-view-card" aria-labelledby={labelledBy} aria-busy={busy || undefined}>
          {children}
        </main>
        {below}
      </div>
    </div>
  );
}

/**
 * A redirect's notice or alert above the card, as the retained auth layout shows its flash: an
 * alert is announced at once, a notice politely.
 */
export function AuthFlash({ flash }: { readonly flash: SignedOutFlash }) {
  const alert = flash.kind === "alert";

  return (
    <div
      className="auth-view-flash"
      data-tone={alert ? "danger" : "success"}
      role={alert ? "alert" : "status"}
    >
      <span className="auth-view-flash-icon">
        <Icon name={alert ? "circle-alert" : "check"} size={12} />
      </span>
      <span>{flash.message}</span>
    </div>
  );
}

/**
 * Calls `restored` when the browser brings the page back from its back/forward cache (Back after
 * leaving for Google or the app), so a form left busy by the departure can be used again.
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
 * Under the sign-in card: the public pages' SPA copies (in a new tab, as the retained page opens
 * them), then the first administrator to ask for help and the version.
 */
export function AuthFooter({ boot }: { readonly boot: SignedOutBootData }) {
  const contact = boot.helpContact;

  return (
    <>
      <nav className="auth-view-links" aria-label="About this workspace">
        <a href="/app/about" target="_blank" rel="noopener">
          About
        </a>
        <a href="/app/privacy" target="_blank" rel="noopener">
          Privacy Policy
        </a>
        <a href="/app/terms" target="_blank" rel="noopener">
          Terms of Service
        </a>
      </nav>
      {contact === null ? null : <AuthHelp contact={contact} version={boot.version} />}
    </>
  );
}

/** The first administrator to ask for help, and the version (the retained `_help_contact`). */
export function AuthHelp({
  contact,
  version,
}: {
  readonly contact: { readonly name: string; readonly emailAddress: string };
  readonly version: string;
}) {
  return (
    <div className="auth-view-help">
      <a href={`mailto:${contact.emailAddress}`} title={`Email ${contact.name}`}>
        <Icon name="life-buoy" size={14} />
        <span>{contact.emailAddress}</span>
      </a>
      <span>Smartfire™ version {version}</span>
    </div>
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
