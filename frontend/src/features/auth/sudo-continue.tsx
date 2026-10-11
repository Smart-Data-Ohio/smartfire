import { useNavigate } from "@tanstack/react-router";
import { useEffect, useRef } from "react";
import { store } from "../../store/store.ts";
import { actions, type GoogleReturn } from "../../sync/runtime.ts";
import { toast } from "../../ui/toast-store.ts";
import { RoutePending } from "../shell/route-pending.tsx";

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
 * says the confirmation is fresh, the writes kept for it go again, each once; then it returns to
 * the page they came from. A refused or stale one changes nothing.
 */
export function SudoContinue() {
  const navigate = useNavigate();
  // Once, even when Strict Mode runs the effect twice: the kept writes are taken on the first.
  const ran = useRef(false);

  useEffect(() => {
    if (ran.current) {
      return;
    }

    ran.current = true;
    actions.confirmation.resumeAfterGoogle().then(
      (result) => {
        announce(result);
        void navigate({ href: result.returnTo, replace: true });
      },
      (error: Error) => {
        toast({ title: "Couldn't finish confirming", description: error.message, tone: "danger" });
        void navigate({ href: "/app/", replace: true });
      },
    );
  }, [navigate]);

  return <RoutePending />;
}
