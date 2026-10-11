import { type KeyboardEvent, useEffect, useRef, useState } from "react";
import type { ChannelWebhook } from "../../gen/ChannelWebhook.ts";
import type { ChannelWebhookCredential } from "../../gen/ChannelWebhookCredential.ts";
import type { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { Avatar } from "../../ui/avatar.tsx";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { Skeleton } from "../../ui/skeleton.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import { copy } from "../admin/admin-parts.tsx";
import { UNKNOWN_NAME, useUser } from "../people/people.ts";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "forbidden" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly webhooks: readonly ChannelWebhook[] };

/** A just-issued URL. It lives only in this component's state, never in the store or storage. */
interface Shown {
  readonly webhookId: number;
  readonly name: string;
  readonly url: string;
}

type Busy =
  | { readonly kind: "create" }
  | { readonly kind: "rename" | "test" | "regenerate" | "delete"; readonly id: number };

const dateFormat = new Intl.DateTimeFormat(undefined, { dateStyle: "medium" });

function formatDate(timestamp: string): string {
  return dateFormat.format(Date.parse(timestamp));
}

function replaced(
  webhooks: readonly ChannelWebhook[],
  webhook: ChannelWebhook,
): readonly ChannelWebhook[] {
  return webhooks.map((item) => (item.id === webhook.id ? webhook : item));
}

function shownFrom(credential: ChannelWebhookCredential): Shown {
  return { webhookId: credential.webhook.id, name: credential.webhook.name, url: credential.url };
}

/** The first validation message for the name, or the failure's own message. */
function nameProblem(failure: ActionError): string {
  const name = failure.fields.name?.[0];

  return name === undefined ? failure.message : `Name ${name}.`;
}

/** The URL just issued, with a copy button, until the viewer is done with it. */
function CredentialNotice({
  shown,
  onDone,
}: {
  readonly shown: Shown;
  readonly onDone: () => void;
}) {
  const code = useRef<HTMLElement>(null);

  return (
    <div className="room-webhook-secret" role="status" aria-label={`${shown.name}'s webhook URL`}>
      <p className="room-webhook-secret-title">Copy the URL for {shown.name} now</p>
      <p className="room-integration-note">
        You won't see this URL again. Anyone who has it can post here as {shown.name}; if it's lost
        or leaked, generate a new one.
      </p>
      <div className="room-webhook-url">
        <code ref={code}>{shown.url}</code>
        <Button
          size="sm"
          variant="secondary"
          icon="copy"
          onClick={() => void copy(shown.url, "Webhook URL", code.current)}
        >
          Copy
        </Button>
      </div>
      <div className="room-integration-actions">
        <Button size="sm" variant="primary" onClick={onDone}>
          Done
        </Button>
      </div>
    </div>
  );
}

interface RowProps {
  readonly webhook: ChannelWebhook;
  readonly busy: Busy | null;
  readonly onRename: (webhook: ChannelWebhook, name: string) => Promise<boolean>;
  readonly onTest: (webhook: ChannelWebhook) => void;
  readonly onRegenerate: (webhook: ChannelWebhook) => void;
  readonly onDelete: (webhook: ChannelWebhook) => void;
}

function WebhookRow({ webhook, busy, onRename, onTest, onRegenerate, onDelete }: RowProps) {
  const creator = useUser(webhook.creatorId);
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState(webhook.name);
  const mine = busy !== null && busy.kind !== "create" && busy.id === webhook.id;
  const locked = busy !== null;

  const save = () => {
    if (locked) return;

    if (name.trim() === webhook.name) {
      setEditing(false);

      return;
    }

    void onRename(webhook, name).then((saved) => {
      if (saved) setEditing(false);
    });
  };

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key !== "Enter") return;

    event.preventDefault();
    save();
  };

  const created = `Created by ${creator?.name ?? UNKNOWN_NAME} on ${formatDate(webhook.createdAt)}`;

  const used =
    webhook.lastUsedAt === null ? "Never used" : `Last used ${formatDate(webhook.lastUsedAt)}`;

  return (
    <li className="room-subscription room-webhook" aria-label={webhook.name}>
      <div className="room-webhook-head">
        <Avatar
          name={webhook.name}
          userId={`webhook-${webhook.id}`}
          size={36}
          decorative
          {...(webhook.avatarUrl === null ? {} : { src: webhook.avatarUrl })}
        />
        <div className="room-webhook-text">
          <span className="room-subscription-name">{webhook.name}</span>
          <span className="room-webhook-meta">
            {created} · {used}
          </span>
        </div>
      </div>
      {editing ? (
        <div className="room-webhook-rename">
          <TextField
            label="Name"
            value={name}
            maxLength={80}
            autoComplete="off"
            spellCheck={false}
            disabled={locked}
            autoFocus
            onKeyDown={onKeyDown}
            onChange={(event) => setName(event.target.value)}
          />
          <div className="room-integration-actions">
            <Button
              size="sm"
              variant="primary"
              disabled={name.trim() === "" || (locked && !mine)}
              loading={mine && busy?.kind === "rename"}
              loadingLabel="Saving…"
              onClick={save}
            >
              Save
            </Button>
            <Button
              size="sm"
              variant="secondary"
              disabled={locked}
              onClick={() => {
                setEditing(false);
                setName(webhook.name);
              }}
            >
              Cancel
            </Button>
          </div>
        </div>
      ) : (
        <div className="room-integration-actions room-webhook-actions">
          <Button
            size="sm"
            variant="secondary"
            icon="pencil"
            aria-label={`Rename ${webhook.name}`}
            disabled={locked}
            onClick={() => {
              setName(webhook.name);
              setEditing(true);
            }}
          >
            Rename
          </Button>
          <Button
            size="sm"
            variant="secondary"
            icon="send"
            aria-label={`Send a test message from ${webhook.name}`}
            disabled={locked && !mine}
            loading={mine && busy?.kind === "test"}
            loadingLabel="Sending…"
            onClick={() => onTest(webhook)}
          >
            Send test
          </Button>
          <Button
            size="sm"
            variant="secondary"
            icon="refresh-cw"
            aria-label={`Generate a new URL for ${webhook.name}`}
            disabled={locked}
            onClick={() => onRegenerate(webhook)}
          >
            New URL
          </Button>
          <Button
            size="sm"
            variant="danger"
            icon="trash"
            aria-label={`Delete ${webhook.name}`}
            disabled={locked}
            onClick={() => onDelete(webhook)}
          >
            Delete
          </Button>
        </div>
      )}
    </li>
  );
}

interface WebhooksSectionProps {
  readonly roomId: number;
  /** The settings dialog is open. Closing it drops a URL still on show, even mid-animation. */
  readonly open: boolean;
}

/**
 * A channel's incoming webhooks (Discord's Integrations → Webhooks), for its creator and
 * administrators, the same people the API lets manage them. Each posts into the room as itself
 * with an APP tag. A webhook's URL carries its token, so it is shown once, right after creating or
 * regenerating, and kept only in this component's state: it goes when the viewer is done with it,
 * leaves the tab, or closes the settings.
 */
export function WebhooksSection({ roomId, open }: WebhooksSectionProps) {
  const mounted = useRef(true);
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [attempt, setAttempt] = useState(0);
  const [busy, setBusy] = useState<Busy | null>(null);
  const [shown, setShown] = useState<Shown | null>(null);
  const [name, setName] = useState("");
  const [nameError, setNameError] = useState<string | undefined>(undefined);
  const [nameAttempt, setNameAttempt] = useState(0);
  const [problem, setProblem] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<ChannelWebhook | null>(null);
  const [regenerating, setRegenerating] = useState<ChannelWebhook | null>(null);

  if (!open && shown !== null) {
    setShown(null);
  }

  useEffect(() => {
    mounted.current = true;

    return () => {
      mounted.current = false;
    };
  }, []);

  // biome-ignore lint/correctness/useExhaustiveDependencies: attempt is the Try again trigger
  useEffect(() => {
    let live = true;

    setLoad({ status: "loading" });
    actions.rooms.channelWebhooks(roomId).then(
      (list) => {
        if (live) setLoad({ status: "ready", webhooks: list.webhooks });
      },
      (failure: ActionError) => {
        if (!live) return;

        setLoad(
          failure.tag === "Forbidden"
            ? { status: "forbidden" }
            : { status: "error", message: failure.message },
        );
      },
    );

    return () => {
      live = false;
    };
  }, [roomId, attempt]);

  const update = (change: (webhooks: readonly ChannelWebhook[]) => readonly ChannelWebhook[]) => {
    setLoad((current) =>
      current.status === "ready"
        ? { status: "ready", webhooks: change(current.webhooks) }
        : current,
    );
  };

  /** Settles a write: false when the section has gone meanwhile and nothing should be shown. */
  const settle = () => {
    if (!mounted.current) return false;

    setBusy(null);

    return true;
  };

  const create = () => {
    if (busy !== null) return;

    if (name.trim() === "") {
      setNameError("Give the webhook a name.");
      setNameAttempt((count) => count + 1);

      return;
    }

    setBusy({ kind: "create" });
    setProblem(null);
    actions.rooms.createChannelWebhook(roomId, { name: name.trim(), avatarSignedId: null }).then(
      (credential) => {
        if (!settle()) return;

        update((webhooks) => [...webhooks, credential.webhook]);
        setName("");
        setNameError(undefined);
        setShown(shownFrom(credential));
      },
      (failure: ActionError) => {
        if (!settle()) return;

        if (failure.tag === "Validation") {
          setNameError(nameProblem(failure));
          setNameAttempt((count) => count + 1);
        } else {
          setProblem(failure.message);
        }
      },
    );
  };

  const rename = (webhook: ChannelWebhook, next: string): Promise<boolean> => {
    if (busy !== null) return Promise.resolve(false);

    setBusy({ kind: "rename", id: webhook.id });
    setProblem(null);

    return actions.rooms.updateChannelWebhook(roomId, webhook.id, { name: next.trim() }).then(
      (saved) => {
        if (!settle()) return false;

        update((webhooks) => replaced(webhooks, saved));
        toast({ title: `Renamed to ${saved.name}`, tone: "success" });

        return true;
      },
      (failure: ActionError) => {
        if (!settle()) return false;

        setProblem(failure.tag === "Validation" ? nameProblem(failure) : failure.message);

        return false;
      },
    );
  };

  const test = (webhook: ChannelWebhook) => {
    if (busy !== null) return;

    setBusy({ kind: "test", id: webhook.id });
    setProblem(null);
    actions.rooms.testChannelWebhook(roomId, webhook.id).then(
      () => {
        toast({ title: `${webhook.name} posted a test message`, tone: "success" });
        settle();
      },
      (failure: ActionError) => {
        if (settle()) setProblem(failure.message);
      },
    );
  };

  const regenerate = () => {
    const target = regenerating;

    if (target === null || busy !== null) return;

    setBusy({ kind: "regenerate", id: target.id });
    setProblem(null);
    actions.rooms.regenerateChannelWebhook(roomId, target.id).then(
      (credential) => {
        if (!settle()) return;

        setRegenerating(null);
        update((webhooks) => replaced(webhooks, credential.webhook));
        setShown(shownFrom(credential));
      },
      (failure: ActionError) => {
        if (!settle()) return;

        setRegenerating(null);
        setProblem(failure.message);
      },
    );
  };

  const remove = () => {
    const target = deleting;

    if (target === null || busy !== null) return;

    setBusy({ kind: "delete", id: target.id });
    setProblem(null);
    actions.rooms.deleteChannelWebhook(roomId, target.id).then(
      () => {
        toast({ title: `Deleted ${target.name}` });

        if (!settle()) return;

        setDeleting(null);
        update((webhooks) => webhooks.filter((item) => item.id !== target.id));
        setShown((current) => (current?.webhookId === target.id ? null : current));
      },
      (failure: ActionError) => {
        if (!settle()) return;

        setDeleting(null);
        setProblem(failure.message);
      },
    );
  };

  if (load.status === "loading") {
    return (
      <div className="room-settings-skeleton" role="status" aria-label="Loading webhooks">
        <Skeleton width="40%" height={12} />
        <Skeleton width="90%" height={12} />
      </div>
    );
  }

  if (load.status === "forbidden") {
    return (
      <p className="room-readonly-note" role="status">
        Only the person who made this channel and administrators can manage its webhooks.
      </p>
    );
  }

  if (load.status === "error") {
    return (
      <div className="room-form-load-error">
        <p className="picker-note picker-error" role="alert">
          Couldn't load webhooks: {load.message}
        </p>
        <Button size="sm" onClick={() => setAttempt((count) => count + 1)}>
          Try again
        </Button>
      </div>
    );
  }

  return (
    <section className="room-integrations" data-room-integration="" aria-label="Webhooks">
      <p className="room-integration-note">
        A webhook gives another service a URL that posts into this channel. Its messages show the
        webhook's name with an APP tag.
      </p>
      {shown === null ? null : <CredentialNotice shown={shown} onDone={() => setShown(null)} />}
      {load.webhooks.length === 0 ? (
        <p className="room-integration-note">No webhooks yet.</p>
      ) : (
        <ul className="room-subscription-list">
          {load.webhooks.map((webhook) => (
            <WebhookRow
              key={webhook.id}
              webhook={webhook}
              busy={busy}
              onRename={rename}
              onTest={test}
              onRegenerate={setRegenerating}
              onDelete={setDeleting}
            />
          ))}
        </ul>
      )}
      {problem === null ? null : (
        <p className="picker-note picker-error" role="alert">
          {problem}
        </p>
      )}
      <section className="room-subscribe" aria-label="New webhook">
        <h3 className="room-subscribe-title">New webhook</h3>
        <TextField
          label="Name"
          placeholder="Deployments"
          autoComplete="off"
          spellCheck={false}
          maxLength={80}
          value={name}
          disabled={busy !== null}
          error={nameError}
          attempt={nameAttempt}
          onKeyDown={(event) => {
            if (event.key !== "Enter") return;

            event.preventDefault();
            create();
          }}
          onChange={(event) => {
            setName(event.target.value);
            setNameError(undefined);
          }}
        />
        <div className="room-integration-actions">
          <Button
            variant="primary"
            icon="plus"
            disabled={busy !== null && busy.kind !== "create"}
            loading={busy?.kind === "create"}
            loadingLabel="Creating…"
            onClick={create}
          >
            Create webhook
          </Button>
        </div>
      </section>
      <Dialog
        open={regenerating !== null}
        onOpenChange={(next) => (busy !== null || next ? undefined : setRegenerating(null))}
        role="alertdialog"
        size="sm"
        title={`New URL for ${regenerating?.name ?? "this webhook"}?`}
        description="The current URL stops working right away. Anything still using it will fail until you give it the new one."
        footer={
          <>
            <Button variant="secondary" onClick={() => setRegenerating(null)} data-autofocus>
              Keep it
            </Button>
            <Button
              variant="danger"
              loading={busy?.kind === "regenerate"}
              loadingLabel="Generating…"
              onClick={regenerate}
            >
              Generate new URL
            </Button>
          </>
        }
      />
      <Dialog
        open={deleting !== null}
        onOpenChange={(next) => (busy !== null || next ? undefined : setDeleting(null))}
        role="alertdialog"
        size="sm"
        title={`Delete ${deleting?.name ?? "this webhook"}?`}
        description="Its URL stops working. Messages it already posted stay in the channel."
        footer={
          <>
            <Button variant="secondary" onClick={() => setDeleting(null)} data-autofocus>
              Keep it
            </Button>
            <Button
              variant="danger"
              loading={busy?.kind === "delete"}
              loadingLabel="Deleting…"
              onClick={remove}
            >
              Delete webhook
            </Button>
          </>
        }
      />
    </section>
  );
}
