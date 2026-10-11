import { useNavigate } from "@tanstack/react-router";
import { type ReactNode, useCallback, useEffect, useRef, useState } from "react";
import type { SudoMethod } from "../../gen/SudoMethod.ts";
import type { SudoSubmission } from "../../gen/SudoSubmission.ts";
import {
  actions,
  afterConfirming,
  type SudoHere,
  type SudoStatus,
  spaReturnPath,
} from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { toast } from "../../ui/toast-store.ts";
import { PageFrame } from "../destinations/page-frame.tsx";
import { RoutePending } from "../shell/route-pending.tsx";
import { pageExit } from "./auth-navigation.ts";
import { type Busy, Choices, TITLE } from "./reauthenticate.tsx";
import "./reauthenticate.css";

/** What the server held for after the confirmation (`sudo::retry`), if anything. */
type Retry = Extract<SudoHere, { kind: "confirmed" }>["retry"];

/** What the page shows. */
type Shown =
  | { readonly kind: "loading" }
  | { readonly kind: "ready"; readonly methods: readonly SudoMethod[]; readonly retry: Retry }
  | { readonly kind: "unreachable"; readonly message: string }
  | { readonly kind: "leaving" };

const LEDE = "What you were doing needs a fresh confirmation. Confirm once, then it continues.";

/**
 * `/app/sudo/new`: the full-page "Confirm it's you", for a page the server's confirmation gate
 * sent here (nothing waits in memory, unlike the dialog). Once confirmed, a page the person
 * asked for (`GET`) loads again; a change is never sent again from here: back to the page it came
 * from to make again. Already confirmed, it goes straight on.
 */
export function SudoNew() {
  const navigate = useNavigate();
  const [shown, setShown] = useState<Shown>({ kind: "loading" });
  const [busy, setBusy] = useState<Busy>(null);
  const [read, setRead] = useState(0);
  // Once per read, even when Strict Mode runs the effect twice.
  const ran = useRef(-1);
  // Set once the page is left (Back, say): an answer that arrives later acts on nothing.
  const gone = useRef(false);

  useEffect(() => {
    gone.current = false;

    return () => {
      gone.current = true;
    };
  }, []);

  useEffect(() => {
    document.title = `${TITLE} · Smartfire`;
  }, []);

  /** Signed out (in another tab, say): on to sign in at `location`, if the page still shows. */
  const signIn = useCallback((location: string) => {
    if (gone.current) {
      return;
    }

    setShown({ kind: "leaving" });
    pageExit.assign(location);
  }, []);

  /** Goes on to what the server held, or home: the confirmation is fresh. */
  const goOn = useCallback(
    (retry: Retry, confirmedHere: boolean) => {
      if (gone.current) {
        return;
      }

      setShown({ kind: "leaving" });

      const { returnTo, follow } = afterConfirming({ kind: "confirmed", retry }, HOME);

      if (follow !== null) {
        // A download leaves this page where it is: it goes back first.
        void navigate({ href: returnTo, replace: true });
        pageExit.assign(follow);

        return;
      }

      if (retry !== null) {
        toast({ title: "Confirmed. Try that again.", tone: "success" });
      } else if (confirmedHere) {
        toast({ title: "Confirmed", tone: "success" });
      }

      void navigate({ href: returnTo, replace: true });
    },
    [navigate],
  );

  const unreachable = useCallback((failure: Error) => {
    if (!gone.current) {
      setShown({ kind: "unreachable", message: failure.message });
    }
  }, []);

  /**
   * What the server said: `taken` once the continuation handed over what it held (the status
   * read only looks, so another tab's Google return keeps what it needs until this page acts).
   */
  const settle = useCallback(
    (status: SudoStatus, taken: boolean) => {
      if (gone.current) {
        return;
      }

      switch (status.kind) {
        case "confirmed":
          if (!taken && status.retry !== null) {
            actions.confirmation.takeHeld().then((held) => settle(held, true), unreachable);

            return;
          }

          goOn(status.retry, false);

          return;
        case "ready":
          setShown({
            kind: "ready",
            methods: status.reauthentication.methods,
            retry: status.reauthentication.retry,
          });

          return;
        case "navigate":
          signIn(status.location);

          return;
        case "error":
        case "unreachable":
          setShown({ kind: "unreachable", message: status.message });
      }
    },
    [goOn, signIn, unreachable],
  );

  useEffect(() => {
    if (ran.current === read) {
      return;
    }

    ran.current = read;
    actions.confirmation.status().then((status) => settle(status, false), unreachable);
  }, [read, settle, unreachable]);

  const submit = async (submission: SudoSubmission) => {
    const reply = await actions.confirmation.confirmHere(submission);

    if (reply.kind === "confirmed") {
      goOn(reply.retry, true);
    } else if (reply.kind === "signedOut") {
      signIn(reply.location);
    }

    return reply.kind === "refused" ? reply.message : null;
  };

  if (shown.kind === "loading" || shown.kind === "leaving") {
    return <RoutePending />;
  }

  if (shown.kind === "unreachable") {
    return (
      <PageFrame title={TITLE} icon="lock" back>
        <div className="reauth-page">
          <p className="text-muted" role="alert">
            Couldn't check your confirmation. {shown.message}
          </p>
          <div className="reauth-page-actions">
            <Button
              variant="primary"
              onClick={() => {
                setShown({ kind: "loading" });
                setRead((count) => count + 1);
              }}
            >
              Try again
            </Button>
            <Button onClick={() => void navigate({ href: HOME, replace: true })}>Go home</Button>
          </div>
        </div>
      </PageFrame>
    );
  }

  const cancel = () => {
    void navigate({ href: spaReturnPath(shown.retry?.returnTo ?? null), replace: true });
  };

  return (
    <PageFrame title={TITLE} icon="lock" back>
      <div className="reauth-page">
        <p className="reauth-page-lede">{LEDE}</p>
        <Focused>
          <Choices
            methods={shown.methods}
            googleNote={null}
            busy={busy}
            onBusy={setBusy}
            onDirty={noteDirty}
            submit={submit}
            startGoogle={actions.confirmation.googleHere}
          />
        </Focused>
        <div className="reauth-page-actions">
          <Button variant="secondary" disabled={busy !== null} onClick={cancel}>
            Cancel
          </Button>
        </div>
      </div>
    </PageFrame>
  );
}

/** The SPA's home, where the page goes when the server held nothing to go back to. */
const HOME = "/app/";

/** A full page has nothing to lose on a stray swipe: what's typed isn't guarded. */
const noteDirty = () => undefined;

/** Puts focus in the first field (the dialog's `data-autofocus`), once the form shows. */
function Focused({ children }: { readonly children: ReactNode }) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const first = ref.current?.querySelector<HTMLElement>("[data-autofocus]");

    (first ?? ref.current?.querySelector<HTMLElement>("button"))?.focus();
  }, []);

  return <div ref={ref}>{children}</div>;
}
