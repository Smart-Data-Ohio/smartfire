import { type KeyboardEvent, useEffect, useState } from "react";
import type { GithubEventChoice } from "../../gen/GithubEventChoice.ts";
import type { GithubSubscription } from "../../gen/GithubSubscription.ts";
import type { GithubSubscriptionList } from "../../gen/GithubSubscriptionList.ts";
import type { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Checkbox } from "../../ui/checkbox.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { Skeleton } from "../../ui/skeleton.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";

interface Draft {
  readonly id: number;
  readonly fullName: string;
  readonly saved: readonly string[];
  readonly events: readonly string[];
}

type Load =
  | { readonly status: "loading" }
  | { readonly status: "forbidden" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly list: GithubSubscriptionList };

function defaultKeys(events: readonly GithubEventChoice[]): string[] {
  const keys: string[] = [];

  for (const event of events) {
    if (event.selectedByDefault) keys.push(event.key);
  }

  return keys;
}

function withEvent(events: readonly string[], key: string, on: boolean): string[] {
  if (!on) {
    const next: string[] = [];

    for (const event of events) {
      if (event !== key) next.push(event);
    }

    return next;
  }

  if (events.includes(key)) return [...events];

  return [...events, key];
}

function sameEvents(left: readonly string[], right: readonly string[]): boolean {
  if (left.length !== right.length) return false;

  for (const event of left) {
    if (!right.includes(event)) return false;
  }

  return true;
}

function draftsFrom(subscriptions: readonly GithubSubscription[]): Draft[] {
  const drafts: Draft[] = [];

  for (const subscription of subscriptions) {
    drafts.push({
      id: subscription.id,
      fullName: subscription.fullName,
      saved: subscription.events,
      events: subscription.events,
    });
  }

  return drafts;
}

function Loading() {
  return (
    <div className="room-settings-skeleton" role="status" aria-label="Loading repositories">
      <Skeleton width="80%" height={12} />
      <Skeleton width="55%" height={12} />
      <Skeleton width="40%" height={32} />
    </div>
  );
}

/**
 * The classic GitHub section: subscribed repositories, their events, and the subscribe form.
 * The creator and administrators see it on every room except a direct message. Anyone else who
 * opens settings gets the refusal the API returns.
 */
export function GithubSubscriptions({ roomId }: { readonly roomId: number }) {
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [attempt, setAttempt] = useState(0);
  const [drafts, setDrafts] = useState<readonly Draft[]>([]);
  const [fullName, setFullName] = useState("");
  const [picked, setPicked] = useState<readonly string[]>([]);
  const [skip, setSkip] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);
  const [showConnect, setShowConnect] = useState(false);
  const [busy, setBusy] = useState<"subscribe" | "save" | "remove" | null>(null);
  const [removing, setRemoving] = useState<Draft | null>(null);

  // biome-ignore lint/correctness/useExhaustiveDependencies: attempt is the Try again trigger
  useEffect(() => {
    let live = true;

    setLoad({ status: "loading" });
    actions.rooms.githubSubscriptions(roomId).then(
      (list) => {
        if (!live) return;

        setLoad({ status: "ready", list });
        setDrafts(draftsFrom(list.subscriptions));
        setPicked(defaultKeys(list.events));
        setFullName("");
        setSkip(false);
        setProblem(null);
        setShowConnect(false);
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

  const list = load.status === "ready" ? load.list : null;

  const fail = (failure: ActionError) => {
    setBusy(null);
    setProblem(failure.message);
    setShowConnect(
      failure.tag === "Validation" &&
        failure.fields.github?.includes("not_linked") === true &&
        (list?.connectPath ?? null) !== null,
    );
  };

  const subscribe = () => {
    if (list === null || busy !== null) return;

    setBusy("subscribe");
    setProblem(null);
    setShowConnect(false);
    actions.rooms
      .subscribeRepository(roomId, {
        fullName,
        events: [...picked],
        skipAccessCheck: skip,
      })
      .then((subscription) => {
        setBusy(null);
        setDrafts((current) => [...current, ...draftsFrom([subscription])]);
        setFullName("");
        setPicked(defaultKeys(list.events));
        setSkip(false);
        toast({ title: `Subscribed to ${subscription.fullName}.`, tone: "success" });
      }, fail);
  };

  const save = (draft: Draft) => {
    if (busy !== null) return;

    setBusy("save");
    setProblem(null);
    actions.rooms
      .updateGithubSubscription(roomId, draft.id, { events: [...draft.events] })
      .then((subscription) => {
        setBusy(null);
        setDrafts((current) => {
          const next: Draft[] = [];

          for (const item of current) {
            next.push(
              item.id === subscription.id
                ? {
                    id: subscription.id,
                    fullName: subscription.fullName,
                    saved: subscription.events,
                    events: subscription.events,
                  }
                : item,
            );
          }

          return next;
        });
        toast({ title: `Subscription to ${subscription.fullName} updated.`, tone: "success" });
      }, fail);
  };

  const remove = () => {
    if (removing === null || busy !== null) return;

    const target = removing;

    setBusy("remove");
    actions.rooms.unsubscribeRepository(roomId, target.id).then(
      () => {
        setBusy(null);
        setRemoving(null);
        setDrafts((current) => {
          const next: Draft[] = [];

          for (const item of current) {
            if (item.id !== target.id) next.push(item);
          }

          return next;
        });
        toast({ title: `Unsubscribed from ${target.fullName}.`, tone: "success" });
      },
      (failure: ActionError) => {
        setBusy(null);
        setRemoving(null);
        fail(failure);
      },
    );
  };

  const onRepositoryKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key !== "Enter") return;

    event.preventDefault();
    subscribe();
  };

  if (load.status === "loading") return <Loading />;

  if (load.status === "forbidden") {
    return (
      <p className="room-readonly-note" role="status">
        Only the person who made this room and administrators can manage GitHub subscriptions.
      </p>
    );
  }

  if (load.status === "error") {
    return (
      <div className="room-form-load-error">
        <p className="picker-note picker-error" role="alert">
          Couldn't load repositories: {load.message}
        </p>
        <Button size="sm" onClick={() => setAttempt((count) => count + 1)}>
          Try again
        </Button>
      </div>
    );
  }

  const catalog = list?.events ?? [];
  const connectPath = list?.connectPath ?? null;

  return (
    <div className="room-integrations" data-room-integration="">
      <p className="room-integration-note">
        Subscribed repositories post pull-request events to this room as the GitHub bot. Your linked
        GitHub account must be able to read the repository, and titles from private repositories
        become visible to the whole room.
      </p>
      {drafts.length === 0 ? (
        <p className="room-integration-note">No repositories subscribed yet.</p>
      ) : (
        <ul className="room-subscription-list">
          {drafts.map((draft) => (
            <li key={draft.id} className="room-subscription">
              <div className="room-subscription-head">
                <span className="room-subscription-name">{draft.fullName}</span>
                <Button
                  size="sm"
                  variant="danger"
                  aria-label={`Remove ${draft.fullName}`}
                  disabled={busy !== null}
                  onClick={() => setRemoving(draft)}
                >
                  Remove
                </Button>
              </div>
              <div className="room-integration-events">
                {catalog.map((event) => (
                  <Checkbox
                    key={event.key}
                    checked={draft.events.includes(event.key)}
                    disabled={busy !== null}
                    label={event.label}
                    onCheckedChange={(on) => {
                      setDrafts((current) => {
                        const next: Draft[] = [];

                        for (const item of current) {
                          next.push(
                            item.id === draft.id
                              ? { ...item, events: withEvent(item.events, event.key, on) }
                              : item,
                          );
                        }

                        return next;
                      });
                    }}
                  />
                ))}
              </div>
              <div className="room-integration-actions">
                <Button
                  size="sm"
                  variant="primary"
                  aria-label={`Save ${draft.fullName}`}
                  disabled={busy !== null || sameEvents(draft.events, draft.saved)}
                  onClick={() => save(draft)}
                >
                  Save
                </Button>
              </div>
            </li>
          ))}
        </ul>
      )}
      <section className="room-subscribe">
        <h3 className="room-subscribe-title">Subscribe a repository</h3>
        <TextField
          label="Repository"
          placeholder="owner/repo"
          autoComplete="off"
          spellCheck={false}
          value={fullName}
          disabled={busy !== null}
          onKeyDown={onRepositoryKeyDown}
          onChange={(event) => {
            setFullName(event.target.value);
            setProblem(null);
            setShowConnect(false);
          }}
        />
        <div className="room-integration-events">
          {catalog.map((event) => (
            <Checkbox
              key={event.key}
              checked={picked.includes(event.key)}
              disabled={busy !== null}
              label={event.label}
              onCheckedChange={(on) => setPicked((current) => withEvent(current, event.key, on))}
            />
          ))}
        </div>
        {list?.administrator === true ? (
          <Checkbox
            checked={skip}
            disabled={busy !== null}
            label="Subscribe without verifying my GitHub access (private pull-request titles stay hidden)"
            onCheckedChange={setSkip}
          />
        ) : null}
        <div className="room-integration-actions">
          <Button
            variant="primary"
            disabled={busy !== null}
            loading={busy === "subscribe"}
            onClick={subscribe}
          >
            Subscribe
          </Button>
        </div>
        {problem === null ? null : (
          <p className="picker-note picker-error" role="alert">
            {problem}
            {showConnect && connectPath !== null ? (
              <>
                {" "}
                <a href={connectPath}>Link your GitHub account</a>
              </>
            ) : null}
          </p>
        )}
      </section>
      <Dialog
        open={removing !== null}
        onOpenChange={(next) => (busy !== null || next ? undefined : setRemoving(null))}
        role="alertdialog"
        size="sm"
        title={`Unsubscribe ${removing?.fullName ?? "this repository"}?`}
        footer={
          <>
            <Button variant="secondary" onClick={() => setRemoving(null)} data-autofocus>
              Keep it
            </Button>
            <Button
              variant="danger"
              loading={busy === "remove"}
              loadingLabel="Removing…"
              onClick={remove}
            >
              Remove
            </Button>
          </>
        }
      />
    </div>
  );
}
