import { useCallback, useEffect, useState } from "react";
import type { SessionInfo } from "../../gen/SessionInfo.ts";
import type { SessionList } from "../../gen/SessionList.ts";
import { ActionError } from "../../sync/run.ts";
import { settings as settingsActions } from "../../sync/settings.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { toast } from "../../ui/toast-store.ts";
import { PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import { useNow } from "../threads/use-now.ts";
import { sessionMeta } from "./settings-format.ts";
import { SettingsGroup, SettingsPage, toastFailure } from "./settings-parts.tsx";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly list: SessionList };

/** What a confirmation is about to do. */
type Pending =
  | { readonly kind: "one"; readonly session: SessionInfo }
  | { readonly kind: "others" };

/**
 * This browser's push subscription, if it has one. Its endpoint goes with the sign-out so the
 * server drops the subscription too (the classic sessions controller does the same); the browser
 * unsubscribes only once the server has signed it out.
 */
async function pushSubscription(): Promise<PushSubscription | null> {
  if (!("serviceWorker" in navigator)) {
    return null;
  }

  const registration = await navigator.serviceWorker.getRegistration(window.location.origin);

  return (await registration?.pushManager.getSubscription()) ?? null;
}

/** Whether a sign-out of this browser went through: the server answers it `Unauthorized`. */
function signedOut(error: Error): boolean {
  return error instanceof ActionError && error.tag === "Unauthorized";
}

/** This browser is signed out: drop its push subscription and leave for the sign-in page. */
function leave(subscription: PushSubscription | null): void {
  const unsubscribed = subscription === null ? Promise.resolve(true) : subscription.unsubscribe();

  void unsubscribed.catch(() => false).then(() => window.location.assign("/session/new"));
}

/**
 * Sessions: every browser signed in to the account, with when each was last active and where,
 * and a way to sign out any one of them or all but this one. Signing this browser out leaves for
 * the sign-in page.
 */
export function SessionsSection() {
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [pending, setPending] = useState<Pending | null>(null);
  const [busy, setBusy] = useState(false);
  const now = useNow();

  const fetchList = useCallback(() => {
    settingsActions.sessions().then(
      (list) => setLoad({ status: "ready", list }),
      (error: Error) => setLoad({ status: "error", message: error.message }),
    );
  }, []);

  useEffect(fetchList, [fetchList]);

  const reload = () => {
    setLoad({ status: "loading" });
    fetchList();
  };

  const landed = (list: SessionList) => {
    setLoad({ status: "ready", list });

    if (list.notice !== null) {
      toast({ title: list.notice, tone: "success" });
    }
  };

  const confirm = () => {
    const action = pending;

    setPending(null);

    if (action === null) {
      return;
    }

    setBusy(true);

    if (action.kind === "one" && action.session.current) {
      void pushSubscription()
        .catch(() => null)
        .then((subscription) =>
          settingsActions.revokeSession(action.session.id, subscription?.endpoint ?? null).then(
            () => leave(subscription),
            (error: Error) => {
              if (signedOut(error)) {
                leave(subscription);
              } else {
                toastFailure("Couldn't sign that session out", error);
                setBusy(false);
              }
            },
          ),
        );

      return;
    }

    const run =
      action.kind === "others"
        ? settingsActions.revokeOtherSessions()
        : settingsActions.revokeSession(action.session.id);

    run
      .then(landed, (error: Error) => toastFailure("Couldn't sign that session out", error))
      .finally(() => setBusy(false));
  };

  return (
    <SettingsPage
      title="Sessions"
      description="Every browser signed in to your account. Signing out takes effect immediately, everywhere."
    >
      {load.status === "loading" ? <PaneListSkeleton rows={3} /> : null}
      {load.status === "error" ? <PaneError message={load.message} onRetry={reload} /> : null}
      {load.status === "ready" ? (
        <SettingsGroup title="Signed in">
          <ul className="settings-list">
            {load.list.sessions.map((session) => (
              <li key={session.id} className="settings-list-row settings-session">
                <span className="settings-list-main">
                  <strong>
                    {session.description}
                    {session.current ? <span className="settings-badge">This device</span> : null}
                  </strong>
                  <span className="text-faint">{sessionMeta(session, now)}</span>
                </span>
                <Button
                  variant="secondary"
                  size="sm"
                  disabled={busy}
                  onClick={() => setPending({ kind: "one", session })}
                >
                  Sign out
                </Button>
              </li>
            ))}
          </ul>
          {load.list.sessions.length > 1 ? (
            <div className="settings-actions">
              <Button
                variant="danger"
                disabled={busy}
                onClick={() => setPending({ kind: "others" })}
              >
                Sign out of all other sessions
              </Button>
            </div>
          ) : null}
        </SettingsGroup>
      ) : null}
      <Dialog
        open={pending !== null}
        onOpenChange={(open) => {
          if (!open) setPending(null);
        }}
        role="alertdialog"
        size="sm"
        title={
          pending?.kind === "others"
            ? "Sign out every other session?"
            : pending?.session.current
              ? "Sign out of this browser?"
              : `Sign out ${pending?.session.description ?? "that session"}?`
        }
        description={
          pending?.kind === "one" && pending.session.current
            ? "You'll go to the sign-in page."
            : "They'll need to sign in again."
        }
        footer={
          <>
            <Button variant="secondary" onClick={() => setPending(null)} data-autofocus>
              Cancel
            </Button>
            <Button variant="danger" onClick={confirm}>
              Sign out
            </Button>
          </>
        }
      />
    </SettingsPage>
  );
}
