import { useEffect, useState } from "react";
import type { InboundEmail } from "../../gen/InboundEmail.ts";
import type { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { Skeleton } from "../../ui/skeleton.tsx";
import { toast } from "../../ui/toast-store.ts";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "forbidden" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly email: InboundEmail };

/**
 * The classic "Email to room" section: the forward-to-room address, or the control that creates
 * it. Shown on every emailable room (not a board, not a direct message) to the creator and
 * administrators. POST always rotates the token; the button is hidden when the workspace has no
 * inbound domain, matching the classic page.
 */
export function InboundEmailSection({ roomId }: { readonly roomId: number }) {
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [attempt, setAttempt] = useState(0);
  const [busy, setBusy] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);

  // biome-ignore lint/correctness/useExhaustiveDependencies: attempt is the Try again trigger
  useEffect(() => {
    let live = true;

    setLoad({ status: "loading" });
    actions.rooms.inboundEmail(roomId).then(
      (email) => {
        if (live) setLoad({ status: "ready", email });
      },
      (failure: ActionError) => {
        if (!live) return;

        if (failure.tag === "Forbidden") {
          setLoad({ status: "forbidden" });

          return;
        }

        setLoad({ status: "error", message: failure.message });
      },
    );

    return () => {
      live = false;
    };
  }, [roomId, attempt]);

  const rotate = () => {
    if (busy) return;

    setBusy(true);
    setProblem(null);
    actions.rooms.rotateInboundEmail(roomId).then(
      (email) => {
        setBusy(false);
        setConfirming(false);
        setLoad({ status: "ready", email });
        toast({ title: "Room email address rotated.", tone: "success" });
      },
      (failure: ActionError) => {
        setBusy(false);
        setConfirming(false);
        setProblem(failure.message);
      },
    );
  };

  if (load.status === "loading") {
    return (
      <div className="room-settings-skeleton" role="status" aria-label="Loading email address">
        <Skeleton width="40%" height={12} />
        <Skeleton width="90%" height={12} />
      </div>
    );
  }

  if (load.status === "forbidden") {
    return (
      <p className="room-readonly-note" role="status">
        Only the person who made this room and administrators can manage its email address.
      </p>
    );
  }

  if (load.status === "error") {
    return (
      <div className="room-form-load-error">
        <p className="picker-note picker-error" role="alert">
          Couldn't load the email address: {load.message}
        </p>
        <Button size="sm" onClick={() => setAttempt((count) => count + 1)}>
          Try again
        </Button>
      </div>
    );
  }

  const email = load.email;
  const address = email.address;

  return (
    <section className="room-integrations" data-room-integration="" aria-label="Email to room">
      <h3 className="room-subscribe-title">Email to room</h3>
      {email.enabled ? (
        address === null ? (
          <>
            <p className="room-integration-note">
              No email address yet. Create one to post forwarded mail in this room.
            </p>
            <div className="room-integration-actions">
              <Button variant="primary" loading={busy} loadingLabel="Creating…" onClick={rotate}>
                Create email address
              </Button>
            </div>
          </>
        ) : (
          <>
            <p className="room-integration-note">
              Forward email to this address to post it in the room. Mail from a member's address
              posts as them; everything else posts as the Email bot with the sender named.
            </p>
            <p className="room-email-address">
              <code>{address}</code>
            </p>
            <div className="room-integration-actions">
              <Button variant="secondary" disabled={busy} onClick={() => setConfirming(true)}>
                Rotate address
              </Button>
            </div>
          </>
        )
      ) : (
        <p className="room-integration-note">
          Inbound email is not configured for this workspace. Set <code>INBOUND_EMAIL_DOMAIN</code>{" "}
          and the relay ingress password to enable forward-to-room.
        </p>
      )}
      {problem === null ? null : (
        <p className="picker-note picker-error" role="alert">
          {problem}
        </p>
      )}
      <Dialog
        open={confirming}
        onOpenChange={(next) => (busy ? undefined : setConfirming(next))}
        role="alertdialog"
        size="sm"
        title="Rotate this room's email address?"
        description="The old address stops working."
        footer={
          <>
            <Button variant="secondary" onClick={() => setConfirming(false)} data-autofocus>
              Keep it
            </Button>
            <Button variant="danger" loading={busy} loadingLabel="Rotating…" onClick={rotate}>
              Rotate address
            </Button>
          </>
        }
      />
    </section>
  );
}
