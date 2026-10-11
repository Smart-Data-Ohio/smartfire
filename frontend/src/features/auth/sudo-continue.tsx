import { Link, useNavigate } from "@tanstack/react-router";
import { useCallback, useEffect, useRef, useState } from "react";
import { store, useStore } from "../../store/store.ts";
import { actions, type GoogleReturn } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { toast } from "../../ui/toast-store.ts";
import { RoutePending } from "../shell/route-pending.tsx";
import "../room/room.css";

/** Tells the person what the Google return came to. */
function announce(result: GoogleReturn): void {
  if (!result.confirmed) {
    // A refused Google confirmation leaves its alert in the boot flash, which the shell shows.
    if (store.getState().boot?.flash == null) {
      toast({
        title: "Google didn't confirm it's you",
        description: "Nothing was changed. Try again.",
        tone: "danger",
      });
    }

    return;
  }

  if (result.abandoned > 0) {
    // What was kept wasn't for this confirmation (another tab's, a new session's) or ran out.
    toast({
      title: result.abandoned === 1 ? "Your change wasn't sent" : "Your changes weren't sent",
      description:
        "The confirmation didn't match the page you left, or came too late. Make the change again.",
      tone: "danger",
    });

    return;
  }

  if (result.failure !== null) {
    toast({
      title: "Confirmed, but the change didn't go through",
      description: result.failure,
      tone: "danger",
    });

    return;
  }

  if (result.replayed > 0) {
    toast({
      title:
        result.replayed === 1
          ? "Confirmed — your change was saved"
          : "Confirmed — your changes were saved",
      tone: "success",
    });

    return;
  }

  if (result.elsewhere > 0) {
    // A duplicated tab sent them: this copy stays quiet rather than send them twice.
    toast({
      title: "Confirmed",
      description:
        result.elsewhere === 1
          ? "Another tab already sent your change."
          : "Another tab already sent your changes.",
      tone: "success",
    });

    return;
  }

  toast({
    title: "Confirmed",
    description:
      result.dropped > 0 ? "Enter the token or secret again to finish." : "Try that again.",
    tone: "success",
  });
}

/** What the continue page shows once it isn't loading. */
type Shown =
  | { readonly kind: "pending"; readonly waiting: boolean }
  | { readonly kind: "stalled" }
  | {
      readonly kind: "held";
      readonly reason: "confirm" | "claim";
      readonly kept: number;
      readonly returnTo: string;
    };

/** What a held change's page says, by why it waits. */
function heldCopy(reason: "confirm" | "claim", kept: number): string {
  const [change, it, hasnt] =
    kept === 1 ? ["your change", "it", "hasn't"] : ["your changes", "them", "haven't"];

  return reason === "confirm"
    ? `The confirmation that came back wasn't the one this page asked for, so ${change} ${hasnt} been sent. Confirm it's you again to send ${it}.`
    : `This browser couldn't make sure only one tab sends ${change}, so ${kept === 1 ? "it" : "they"} ${hasnt} been sent. Try again, or discard ${it} and make ${it} again.`;
}

/**
 * `/app/sudo/continue`, where a Google confirmation started in the SPA comes back. Once the server
 * says it confirmed the attempt this tab kept writes for, they go again, each once; then it returns
 * to the page they came from. Otherwise the writes stay kept and the page asks to confirm again
 * (or, where this tab couldn't claim them so no other tab sends them too, to try claiming again),
 * until the person does or discards them. When the server can't be asked (or its answer is lost),
 * the page offers to try again: the server answers for the attempt again.
 */
export function SudoContinue() {
  const navigate = useNavigate();
  // The hand-off belongs to whoever chose Google: wait for the boot to say who's signed in.
  const viewerId = useStore((state) => state.me?.user.id ?? state.boot?.user.id ?? null);
  const [shown, setShown] = useState<Shown>({ kind: "pending", waiting: false });
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [attempt, setAttempt] = useState(0);
  // Once per attempt, even when Strict Mode runs the effect twice.
  const ran = useRef(-1);
  // Aborted by Discard, or by leaving the page: stops waiting for another tab, sends nothing more.
  const stop = useRef(new AbortController());
  const mounted = useRef(false);
  const kept = actions.confirmation.kept();

  useEffect(() => {
    mounted.current = true;

    return () => {
      mounted.current = false;
      // Strict Mode mounts again at once; only a page really left stops.
      queueMicrotask(() => {
        if (!mounted.current) stop.current.abort();
      });
    };
  }, []);

  const finish = useCallback(
    (result: GoogleReturn) => {
      if (stop.current.signal.aborted) {
        return;
      }

      if (result.unreachable !== null) {
        setShown({ kind: "stalled" });

        return;
      }

      if (result.held !== null) {
        setShown({
          kind: "held",
          reason: result.held,
          kept: result.kept,
          returnTo: result.returnTo,
        });

        return;
      }

      announce(result);
      void navigate({ href: result.returnTo, replace: true });
    },
    [navigate],
  );

  useEffect(() => {
    if (viewerId === null || ran.current === attempt) {
      return;
    }

    ran.current = attempt;
    actions.confirmation
      .resumeAfterGoogle({
        signal: stop.current.signal,
        onWait: () => setShown({ kind: "pending", waiting: true }),
      })
      .then(finish, () => setShown({ kind: "stalled" }));
  }, [viewerId, attempt, finish]);

  /** Gives up on what's kept, whatever this page is waiting on: nothing more is sent. */
  const discard = (returnTo: string) => {
    stop.current.abort();
    actions.confirmation.discardKept();
    void navigate({ href: returnTo, replace: true });
  };

  const discardButton = (count: number, returnTo: string) => (
    <Button onClick={() => discard(returnTo)}>
      {count === 1 ? "Discard change" : "Discard changes"}
    </Button>
  );

  if (shown.kind === "pending") {
    if (kept === null) {
      return <RoutePending />;
    }

    const change = kept.writes === 1 ? "change" : "changes";

    return (
      <section className="room room-error enter-fade" aria-label="Confirmation" aria-busy="true">
        <p className="text-title">
          {shown.waiting ? `Another tab is finishing this ${change}` : `Finishing your ${change}`}
        </p>
        <p className="text-muted" role="status">
          {shown.waiting
            ? "That tab may still send it. This page goes on once it's done; discarding here stops only this page."
            : "Checking your confirmation…"}
        </p>
        <div className="room-error-actions">{discardButton(kept.writes, kept.returnTo)}</div>
      </section>
    );
  }

  if (shown.kind === "stalled") {
    return (
      <section className="room room-error enter-fade" aria-label="Confirmation">
        <p className="text-title">Couldn't finish confirming</p>
        <p className="text-muted" role="alert">
          The server didn't answer, so your change hasn't been saved yet. It's still waiting: try
          again to finish it.
        </p>
        <div className="room-error-actions">
          <Button
            variant="primary"
            onClick={() => {
              setShown({ kind: "pending", waiting: false });
              setAttempt((count) => count + 1);
            }}
          >
            Try again
          </Button>
          {kept === null ? null : discardButton(kept.writes, kept.returnTo)}
          <Link to="/">Go home</Link>
        </div>
      </section>
    );
  }

  const { reason, kept: count, returnTo } = shown;

  const proceed = () => {
    setBusy(true);
    setError(null);

    if (reason === "claim") {
      actions.confirmation.resumeAfterGoogle({ signal: stop.current.signal }).then(
        (result) => {
          setBusy(false);
          finish(result);
        },
        () => {
          setBusy(false);
          setShown({ kind: "stalled" });
        },
      );

      return;
    }

    actions.confirmation.confirmAgain().then(
      (start) => {
        // Discarded meanwhile: nothing is left to confirm.
        if (stop.current.signal.aborted) {
          return;
        }

        if (start.kind === "navigate") {
          window.location.assign(start.location);

          return;
        }

        setBusy(false);
        setError(start.message);
      },
      () => {
        setBusy(false);
        setError("Google confirmation didn't start. Try again.");
      },
    );
  };

  return (
    <section className="room room-error enter-fade" aria-label="Confirmation">
      <p className="text-title">
        {count === 1 ? "Your change hasn't been sent" : "Your changes haven't been sent"}
      </p>
      <p className="text-muted" role="alert">
        {error ?? heldCopy(reason, count)}
      </p>
      <div className="room-error-actions">
        <Button variant="primary" disabled={busy} onClick={proceed}>
          {reason === "claim" ? "Try again" : "Confirm again"}
        </Button>
        {discardButton(count, returnTo)}
      </div>
    </section>
  );
}
