import { useBlocker, useRouter, useSearch } from "@tanstack/react-router";
import { type FormEvent, useCallback, useEffect, useId, useRef, useState } from "react";
import { useKeyboardInset } from "../../lib/keyboard-inset.ts";
import { isSpaPath, spaUrlFor } from "../../lib/screens.ts";
import { type SetupAnswer, type SetupState, twoFactorSetup } from "../../sync/two-factor-setup.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { Skeleton } from "../../ui/skeleton.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { Toaster } from "../../ui/toast.tsx";
import { toast } from "../../ui/toast-store.ts";
import { pushSubscription } from "../settings/sessions-section.tsx";
import { svgDataUrl } from "../settings/settings-format.ts";
import "./two-factor-setup.css";

/** Classic pages the SPA shows under another name, as the server's own return to the SPA maps them. */
const SPA_HOMES = new Map([
  ["/", "/app/"],
  ["/users/me/profile", "/app/settings"],
]);

/** Where a server destination opens: its SPA page when the SPA has one, else the URL itself. */
export function spaDestination(url: string): string {
  const parsed = new URL(url, window.location.origin);

  if (parsed.origin !== window.location.origin) {
    return url;
  }

  const home = SPA_HOMES.get(parsed.pathname);

  if (home !== undefined) {
    return home;
  }

  if (isSpaPath(parsed.pathname)) {
    return `${parsed.pathname}${parsed.search}${parsed.hash}`;
  }

  return spaUrlFor(parsed.pathname, parsed.search) ?? url;
}

/** The setup expired and the server made a new key: the old one's codes no longer count. */
export const EXPIRED_MESSAGE =
  "This setup expired, so it has a new key. Add the new key to your app, then enter its code.";

const EMPTY_CODE = "Enter the 6-digit code from your app.";

type Screen =
  | { readonly step: "loading" }
  | { readonly step: "failed"; readonly message: string }
  | {
      readonly step: "ready";
      readonly setup: SetupState;
      readonly error: string | undefined;
      readonly attempt: number;
    }
  | {
      readonly step: "codes";
      readonly codes: readonly string[];
      readonly signedOut: number;
      readonly continueUrl: string;
    }
  | { readonly step: "leaving" };

/**
 * Setting up two-step sign-in, for the signed-in person outside the app's shell: the QR code
 * and the manual key for an authenticator app, the first code, then the backup codes, shown once.
 * Settings opens it; so does the API's enrollment gate (`required`), which leaves signing out as
 * the only other way on. Once set up, the page given in `return_to` (else the server's) loads in
 * full, so the app starts over on the verified session. A late answer after the screen has gone
 * is ignored.
 */
export function TwoFactorSetupPage() {
  const search = useSearch({ from: "/two_factor/setup" });
  const router = useRouter();
  const titleId = useId();
  const [screen, setScreen] = useState<Screen>({ step: "loading" });
  const [busy, setBusy] = useState<"confirm" | "signOut" | null>(null);
  const alive = useRef(true);

  // The view fills the visible viewport, which an overlaid (iOS) keyboard shrinks while viewport
  // units hold, so it follows the keyboard as the shell and the sign-in pages do.
  useKeyboardInset();

  useEffect(() => {
    alive.current = true;
    document.title = "Set up two-step sign-in · Smartfire";

    return () => {
      alive.current = false;
    };
  }, []);

  const leave = useCallback((url: string) => {
    setScreen({ step: "leaving" });
    twoFactorSetup.leave(url);
  }, []);

  /** Shows an answer: the setup (with its refusal), the codes, or the page to go to instead. */
  const land = useCallback(
    (answer: SetupAnswer, previous: SetupState | null, attempt: number) => {
      switch (answer.kind) {
        case "ready":
          setScreen({ step: "ready", setup: answer.setup, error: undefined, attempt });

          return;
        case "error": {
          const expired = previous !== null && previous.secret !== answer.setup.secret;

          setScreen({
            step: "ready",
            setup: answer.setup,
            error: expired ? EXPIRED_MESSAGE : answer.message,
            attempt,
          });

          return;
        }

        case "recoveryCodes":
          setScreen({
            step: "codes",
            codes: answer.codes,
            signedOut: answer.signedOut,
            continueUrl: answer.continueUrl,
          });

          return;
        case "navigate":
          leave(spaDestination(answer.location));

          return;
      }
    },
    [leave],
  );

  const load = useCallback(() => {
    setScreen({ step: "loading" });
    twoFactorSetup.read().then(
      (answer) => {
        if (alive.current) land(answer, null, 0);
      },
      (error: Error) => {
        if (alive.current) setScreen({ step: "failed", message: error.message });
      },
    );
  }, [land]);

  useEffect(load, [load]);

  const confirm = (code: string, setup: SetupState, attempt: number) => {
    if (busy !== null) return;

    if (code.trim() === "") {
      setScreen({ step: "ready", setup, error: EMPTY_CODE, attempt: attempt + 1 });

      return;
    }

    setBusy("confirm");
    twoFactorSetup
      .confirm(code)
      .then(
        (answer) => {
          if (alive.current) land(answer, setup, attempt + 1);
        },
        (error: Error) => {
          if (alive.current) {
            setScreen({ step: "ready", setup, error: error.message, attempt: attempt + 1 });
          }
        },
      )
      .finally(() => {
        if (alive.current) setBusy(null);
      });
  };

  const signOut = () => {
    if (busy !== null) return;

    setBusy("signOut");

    void (async () => {
      const subscription = await pushSubscription().catch(() => null);

      try {
        const answer = await twoFactorSetup.signOut(subscription?.endpoint ?? null);

        // The server has dropped the subscription with the session; the browser follows.
        await subscription?.unsubscribe().catch(() => false);

        if (!alive.current) return;

        leave(answer.kind === "navigate" || answer.kind === "signedIn" ? answer.location : "/");
      } catch (error) {
        if (!alive.current) return;

        setBusy(null);
        toast({
          title: "Couldn't sign out",
          description: error instanceof Error ? error.message : "Try again.",
          tone: "danger",
        });
      }
    })();
  };

  const finish = (continueUrl: string) => {
    leave(search.return_to ?? spaDestination(continueUrl));
  };

  const back =
    search.required === true || search.return_to === undefined
      ? null
      : () => router.history.push(search.return_to ?? "/app/");

  const signOutButton = (
    <Button
      variant="ghost"
      className="two-factor-centre"
      loading={busy === "signOut"}
      loadingLabel="Signing out…"
      onClick={signOut}
    >
      Sign out
    </Button>
  );

  return (
    <div className="two-factor-view">
      <main
        className="two-factor-card"
        aria-labelledby={titleId}
        aria-busy={screen.step === "loading" || screen.step === "leaving" || undefined}
      >
        {screen.step === "codes" ? (
          <CodesStep
            titleId={titleId}
            codes={screen.codes}
            signedOut={screen.signedOut}
            onDone={() => finish(screen.continueUrl)}
          />
        ) : (
          <>
            <header className="two-factor-header">
              <Title id={titleId} focusOnMount={screen.step === "failed"}>
                Set up two-step sign-in
              </Title>
              <p className="two-factor-lede">
                Everyone signs in with an authenticator app, on top of their password or Google
                sign-in.
              </p>
            </header>
            {screen.step === "ready" ? (
              <SetupStep
                setup={screen.setup}
                error={screen.error}
                attempt={screen.attempt}
                busy={busy === "confirm"}
                onConfirm={(code) => confirm(code, screen.setup, screen.attempt)}
              />
            ) : null}
            {screen.step === "failed" ? (
              <div className="two-factor-failed" role="alert">
                <p>{screen.message}</p>
                <Button variant="secondary" size="sm" icon="rotate-ccw" onClick={load}>
                  Try again
                </Button>
              </div>
            ) : null}
            {screen.step === "loading" || screen.step === "leaving" ? <LoadingStep /> : null}
            <div className="two-factor-stack">
              {back === null ? null : (
                <Button variant="secondary" className="two-factor-wide" onClick={back}>
                  Back to settings
                </Button>
              )}
              {signOutButton}
            </div>
          </>
        )}
      </main>
      <Toaster />
    </div>
  );
}

/** The card's heading; focus lands on it when a step replaces the card's content. */
function Title({
  id,
  focusOnMount,
  children,
}: {
  readonly id: string;
  readonly focusOnMount: boolean;
  readonly children: string;
}) {
  const ref = useRef<HTMLHeadingElement | null>(null);

  useEffect(() => {
    if (focusOnMount) ref.current?.focus();
  }, [focusOnMount]);

  return (
    <h1 ref={ref} id={id} className="two-factor-title" tabIndex={-1}>
      {children}
    </h1>
  );
}

/** The steps' shape while the key loads: the QR plate and the key, so nothing moves after. */
function LoadingStep() {
  return (
    <div className="two-factor-loading" role="status" aria-label="Loading your setup">
      <Skeleton width="80%" height={14} />
      <Skeleton width={192} height={192} radius="md" />
      <Skeleton width="60%" height={14} />
      <Skeleton width={200} height={24} />
    </div>
  );
}

interface SetupStepProps {
  readonly setup: SetupState;
  readonly error: string | undefined;
  readonly attempt: number;
  readonly busy: boolean;
  readonly onConfirm: (code: string) => void;
}

/** The three steps: add an account, scan the code (or type the key), enter the first code. */
function SetupStep({ setup, error, attempt, busy, onConfirm }: SetupStepProps) {
  const [code, setCode] = useState("");
  const field = useRef<HTMLInputElement | null>(null);

  // A refusal empties the field and puts the cursor back in it, ready for the next code.
  const [seenAttempt, setSeenAttempt] = useState(attempt);

  if (seenAttempt !== attempt) {
    setSeenAttempt(attempt);
    setCode("");
  }

  useEffect(() => {
    if (attempt > 0) field.current?.focus();
  }, [attempt]);

  // On a desktop the code field is where typing starts; a phone keeps its keyboard down so the
  // steps above stay in view.
  useEffect(() => {
    if ("matchMedia" in window && window.matchMedia("(pointer: fine)").matches) {
      field.current?.focus();
    }
  }, []);

  const copyKey = () => {
    navigator.clipboard.writeText(setup.secret).then(
      () => toast({ title: "Key copied", tone: "success" }),
      (failure: Error) =>
        toast({ title: "Couldn't copy the key", description: failure.message, tone: "danger" }),
    );
  };

  const submit = (event: FormEvent) => {
    event.preventDefault();
    onConfirm(code);
  };

  return (
    <>
      <ol className="two-factor-steps">
        <li>
          Open your authenticator app (Google Authenticator, 1Password, or Authy) and add an
          account.
        </li>
        <li>
          Scan this code:
          <img
            className="two-factor-qr"
            src={svgDataUrl(setup.qrSvg)}
            alt="QR code for your authenticator app"
            width={192}
            height={192}
          />
          {setup.otpauthUri.startsWith("otpauth://") ? (
            <a
              className="button two-factor-open-app"
              data-variant="secondary"
              data-size="sm"
              href={setup.otpauthUri}
            >
              Open in your authenticator app
            </a>
          ) : null}
          <span className="two-factor-hint">Can't scan? Enter this key manually:</span>
          <span className="two-factor-key-row">
            <code className="two-factor-key" data-manual-key>
              {setup.manualKey}
            </code>
            <Button variant="ghost" size="sm" icon="copy" onClick={copyKey}>
              Copy key
            </Button>
          </span>
        </li>
        <li>Enter the 6-digit code from the app:</li>
      </ol>
      <form className="two-factor-form" onSubmit={submit} noValidate>
        <TextField
          ref={field}
          label="Authenticator code"
          className="two-factor-code"
          value={code}
          readOnly={busy}
          inputMode="numeric"
          autoComplete="one-time-code"
          pattern="[0-9 ]*"
          maxLength={10}
          placeholder="123 456"
          spellCheck={false}
          error={error}
          attempt={attempt}
          onChange={(event) => setCode(event.target.value)}
        />
        <Button
          type="submit"
          variant="primary"
          size="lg"
          className="two-factor-wide"
          loading={busy}
          loadingLabel="Verifying…"
        >
          Verify and continue
        </Button>
      </form>
    </>
  );
}

interface CodesStepProps {
  readonly titleId: string;
  readonly codes: readonly string[];
  readonly signedOut: number;
  readonly onDone: () => void;
}

/**
 * The backup codes, shown this once: copy them, download them, then say they're saved. Until then
 * leaving asks first, since nobody can show them again: closing or reloading the tab through the
 * browser's own prompt, and Back or any other move within the app through a confirmation.
 */
function CodesStep({ titleId, codes, signedOut, onDone }: CodesStepProps) {
  const text = codes.join("\n");
  const saved = useRef(false);

  const leaving = useBlocker({
    shouldBlockFn: () => !saved.current,
    // The unload listener below asks for a reload or a closed tab.
    enableBeforeUnload: false,
    withResolver: true,
  });

  useEffect(() => {
    const warn = (event: BeforeUnloadEvent) => {
      if (!saved.current) event.preventDefault();
    };

    window.addEventListener("beforeunload", warn);

    return () => window.removeEventListener("beforeunload", warn);
  }, []);

  const done = () => {
    saved.current = true;
    onDone();
  };

  const copy = () => {
    navigator.clipboard.writeText(text).then(
      () => toast({ title: "Backup codes copied", tone: "success" }),
      (failure: Error) =>
        toast({ title: "Couldn't copy the codes", description: failure.message, tone: "danger" }),
    );
  };

  const download = () => {
    const link = document.createElement("a");

    link.href = URL.createObjectURL(new Blob([`${text}\n`], { type: "text/plain" }));
    link.download = "smartfire-backup-codes.txt";
    link.click();
    URL.revokeObjectURL(link.href);
  };

  return (
    <>
      <header className="two-factor-header">
        <Title id={titleId} focusOnMount>
          Save your backup codes
        </Title>
        <p className="two-factor-lede">
          Each code signs you in once if you lose your authenticator. They will not be shown again.
        </p>
        {signedOut > 0 ? <p className="two-factor-note">Signed out your other devices</p> : null}
      </header>
      <ul className="two-factor-codes" aria-label="Backup codes">
        {codes.map((code) => (
          <li key={code}>
            <code>{code}</code>
          </li>
        ))}
      </ul>
      <div className="two-factor-stack">
        <div className="two-factor-row">
          <Button variant="secondary" icon="copy" onClick={copy}>
            Copy all
          </Button>
          <Button variant="secondary" icon="download" onClick={download}>
            Download
          </Button>
        </div>
        <Button variant="primary" size="lg" className="two-factor-wide" onClick={done}>
          I've saved these codes
        </Button>
      </div>
      <Dialog
        open={leaving.status === "blocked"}
        onOpenChange={(open) => {
          if (!open) leaving.reset?.();
        }}
        role="alertdialog"
        size="sm"
        title="Leave without your backup codes?"
        description="They won't be shown again. Copy or download them first if you haven't."
        footer={
          <>
            <Button variant="secondary" onClick={() => leaving.reset?.()} data-autofocus>
              Stay
            </Button>
            <Button variant="danger" onClick={() => leaving.proceed?.()}>
              Leave without them
            </Button>
          </>
        }
      />
    </>
  );
}

export default TwoFactorSetupPage;
