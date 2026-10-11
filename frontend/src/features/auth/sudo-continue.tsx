import { Link, useNavigate } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";
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
      description: "The confirmation didn't match the page you left. Make the change again.",
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

  toast({
    title: "Confirmed",
    description:
      result.dropped > 0 ? "Enter the token or secret again to finish." : "Try that again.",
    tone: "success",
  });
}

/**
 * `/app/sudo/continue`, where a Google confirmation started in the SPA comes back. Once the server
 * says it confirmed the attempt this tab kept writes for, they go again, each once; then it returns
 * to the page they came from. Anything else changes nothing, and says the change wasn't sent. When
 * the server can't be asked (or its answer is lost), the writes stay kept and the page offers to
 * try again: the server answers for the attempt again.
 */
export function SudoContinue() {
  const navigate = useNavigate();
  // The hand-off belongs to whoever chose Google: wait for the boot to say who's signed in.
  const viewerId = useStore((state) => state.me?.user.id ?? state.boot?.user.id ?? null);
  // The server couldn't say whether the confirmation is fresh: the writes wait for another try.
  const [stalled, setStalled] = useState(false);
  const [attempt, setAttempt] = useState(0);
  // Once per attempt, even when Strict Mode runs the effect twice.
  const ran = useRef(-1);

  useEffect(() => {
    if (viewerId === null || ran.current === attempt) {
      return;
    }

    ran.current = attempt;
    actions.confirmation.resumeAfterGoogle().then(
      (result) => {
        if (result.unreachable !== null) {
          setStalled(true);

          return;
        }

        announce(result);
        void navigate({ href: result.returnTo, replace: true });
      },
      () => setStalled(true),
    );
  }, [viewerId, attempt, navigate]);

  if (!stalled) {
    return <RoutePending />;
  }

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
            setStalled(false);
            setAttempt((count) => count + 1);
          }}
        >
          Try again
        </Button>
        <Link to="/">Go home</Link>
      </div>
    </section>
  );
}
