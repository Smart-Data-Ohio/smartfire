import { Link } from "@tanstack/react-router";
import { type FormEvent, useCallback, useEffect, useRef, useState } from "react";
import type { PersonalAgent } from "../../gen/PersonalAgent.ts";
import type { PersonalAgentAccess } from "../../gen/PersonalAgentAccess.ts";
import type { PersonalAgentCredential } from "../../gen/PersonalAgentCredential.ts";
import type { PersonalAgentList } from "../../gen/PersonalAgentList.ts";
import type { UpdatePersonalAgent } from "../../gen/UpdatePersonalAgent.ts";
import { ActionError } from "../../sync/run.ts";
import { meAgents, SAVE_UNCONFIRMED } from "../../sync/settings.ts";
import { AgentAvatar } from "../../ui/agent-avatar.tsx";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import { Confirm, takeDraft } from "../admin/admin-parts.tsx";
import { PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import { timeAgo } from "../threads/thread-format.ts";
import { AccessFields, ExpiryField, IdentityFields, SecretBlock } from "./agent-parts.tsx";
import { AgentWizard, WIZARD_DRAFT_KEY } from "./agent-wizard.tsx";
import {
  type AgentCache,
  nextSequence,
  withAgent,
  withCreated,
  withList,
  withRemoved,
  withTokens,
} from "./agents-cache.ts";
import {
  abilityLabel,
  agentIcon,
  canCreate,
  clientOf,
  type Expiry,
  emptyDraft,
  expiresAtOf,
  parseDraft,
  roomNames,
  shortDay,
  sortedTokens,
  TOKEN_STATE,
  tokenMeta,
  turnedOff,
  usage,
  type WizardDraft,
} from "./agents-format.ts";
import {
  FieldError,
  fieldsOf,
  SettingsGroup,
  SettingsPage,
  toastFailure,
  useBusy,
} from "./settings-parts.tsx";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly cache: AgentCache };

/** A token being issued or rotated: which agent, and the token it replaces. */
interface TokenAsk {
  readonly agent: PersonalAgent;
  readonly replacing: PersonalAgentCredential | null;
}

/** A secret to show once, for an agent. */
interface Shown {
  readonly agent: PersonalAgent;
  readonly secret: string;
}

type Ask = {
  readonly title: string;
  readonly message: string;
  readonly label: string;
  readonly danger: boolean;
  readonly run: () => void;
} | null;

/** Whether an agent edit got no answer in time, so it may or may not have landed. */
function saveUnconfirmed(error: Error): boolean {
  return error instanceof ActionError && error.tag === SAVE_UNCONFIRMED;
}

/** `held` with the tokens in `incoming` in place of their old copies, and any new ones first. */
function mergedTokens(
  held: readonly PersonalAgentCredential[],
  incoming: readonly PersonalAgentCredential[],
): PersonalAgentCredential[] {
  return [...incoming, ...held.filter((h) => !incoming.some((i) => i.id === h.id))];
}

/**
 * The edit dialog's writes and reload, each landing in the section's cache in the order they were
 * sent (see agents-cache.ts), and answering what the server said.
 */
interface AgentWrites {
  readonly rename: (
    agentId: number,
    change: Partial<UpdatePersonalAgent>,
  ) => Promise<PersonalAgent>;
  readonly setAccess: (agentId: number, access: PersonalAgentAccess) => Promise<PersonalAgent>;
  readonly reload: () => Promise<void>;
}

/** One token: its name and dates, and Rotate and Revoke while it can still be used. */
function TokenRow({
  credential,
  enabled,
  busy,
  onRotate,
  onRevoke,
}: {
  readonly credential: PersonalAgentCredential;
  readonly enabled: boolean;
  readonly busy: boolean;
  readonly onRotate: () => void;
  readonly onRevoke: () => void;
}) {
  const now = Date.now();

  return (
    <li className="settings-list-row agents-token" data-state={credential.state}>
      <span className="settings-list-main">
        <strong>
          {credential.name} <code className="agents-last-four">…{credential.lastFour}</code>
          {credential.state === "active" ? null : (
            <span className="settings-badge">{TOKEN_STATE[credential.state]}</span>
          )}
        </strong>
        <span className="text-faint">{tokenMeta(credential, (at) => timeAgo(at, now))}</span>
      </span>
      {credential.state === "revoked" ? null : (
        <span className="agents-token-actions">
          {enabled ? (
            <Button
              variant="secondary"
              size="sm"
              disabled={busy}
              onClick={onRotate}
              aria-label={`Rotate ${credential.name}`}
            >
              Rotate
            </Button>
          ) : null}
          <Button
            variant="danger"
            size="sm"
            disabled={busy}
            onClick={onRevoke}
            aria-label={`Revoke ${credential.name}`}
          >
            Revoke
          </Button>
        </span>
      )}
    </li>
  );
}

/** One agent: who it is, where it can go, what it can do, and its tokens. */
function AgentCard({
  agent,
  list,
  busy,
  onEdit,
  onRemove,
  onIssue,
  onRotate,
  onRevoke,
}: {
  readonly agent: PersonalAgent;
  readonly list: PersonalAgentList;
  readonly busy: (key: string) => boolean;
  readonly onEdit: () => void;
  readonly onRemove: () => void;
  readonly onIssue: () => void;
  readonly onRotate: (credential: PersonalAgentCredential) => void;
  readonly onRevoke: (credential: PersonalAgentCredential) => void;
}) {
  const client = clientOf(agent);
  const rooms = roomNames(list, agent.roomIds);
  const now = Date.now();

  return (
    <article className="agents-card" aria-labelledby={`agent-${agent.id}-name`}>
      <header className="agents-card-header">
        <AgentAvatar
          seed={`${agent.userId}`}
          name={agent.name}
          icon={agentIcon(agent)}
          size={40}
          decorative
          {...(agent.suspended || !list.enabled ? { state: "muted" as const } : {})}
        />
        <span className="agents-card-title">
          <h3 id={`agent-${agent.id}-name`} className="text-title">
            {agent.name}
            {agent.suspended ? <span className="settings-badge">Suspended</span> : null}
            {!agent.suspended && !list.enabled ? <span className="settings-badge">Off</span> : null}
          </h3>
          <span className="text-faint">
            {client === null ? "Agent" : client.label}
            {` · added ${shortDay(agent.createdAt)} · `}
            {agent.lastUsedAt === null
              ? "never used"
              : `last used ${timeAgo(agent.lastUsedAt, now)}`}
          </span>
        </span>
      </header>
      <dl className="agents-facts">
        <div>
          <dt className="settings-label">Rooms</dt>
          <dd>{rooms.length === 0 ? "Only your direct messages with it" : rooms.join(", ")}</dd>
        </div>
        <div>
          <dt className="settings-label">Abilities</dt>
          <dd>
            {list.enabled
              ? agent.capabilities.length === 0 || rooms.length === 0
                ? "Chat with you in direct messages"
                : agent.capabilities.map(abilityLabel).join(", ")
              : "None while personal agents are off"}
          </dd>
        </div>
      </dl>
      <section className="agents-tokens" aria-label={`${agent.name}'s tokens`}>
        <h4 className="settings-label">Tokens</h4>
        {agent.credentials.length === 0 ? (
          <p className="text-muted">No tokens. It can't connect until you make one.</p>
        ) : (
          <ul className="settings-list">
            {sortedTokens(agent.credentials).map((credential) => (
              <TokenRow
                key={credential.id}
                credential={credential}
                enabled={list.enabled}
                busy={busy(`token-${credential.id}`)}
                onRotate={() => onRotate(credential)}
                onRevoke={() => onRevoke(credential)}
              />
            ))}
          </ul>
        )}
      </section>
      <div className="settings-actions agents-card-actions">
        {list.enabled ? (
          <>
            <Button
              variant="secondary"
              size="sm"
              icon="plus"
              onClick={onIssue}
              aria-label={`New token for ${agent.name}`}
            >
              New token
            </Button>
            <Button
              variant="secondary"
              size="sm"
              icon="pencil"
              onClick={onEdit}
              aria-label={`Edit ${agent.name}`}
            >
              Edit
            </Button>
          </>
        ) : null}
        <Button
          variant="ghost"
          size="sm"
          icon="trash"
          disabled={busy(`agent-${agent.id}`)}
          onClick={onRemove}
          aria-label={`Remove ${agent.name}`}
        >
          Remove
        </Button>
      </div>
    </article>
  );
}

/** A new token for an agent, or a replacement for one: its name (when new) and expiry. */
function TokenDialog({
  ask,
  onClose,
  onIssued,
  onTurnedOff,
}: {
  readonly ask: TokenAsk | null;
  readonly onClose: () => void;
  /** The workspace turned personal agents off since the list loaded: load it again. */
  readonly onTurnedOff: () => void;
  /** `at`: the request's number (agents-cache.ts), taken when it was sent. */
  readonly onIssued: (
    agent: PersonalAgent,
    credential: PersonalAgentCredential,
    secret: string,
    at: number,
  ) => void;
}) {
  const [name, setName] = useState("");
  const [expiry, setExpiry] = useState<Expiry>("90d");
  const [error, setError] = useState<string | undefined>(undefined);
  const { busy, track } = useBusy();
  const opened = ask?.agent.id;

  useEffect(() => {
    if (opened !== undefined) {
      setName("");
      setExpiry("90d");
      setError(undefined);
    }
  }, [opened]);

  if (ask === null) {
    return <Dialog open={false} onOpenChange={onClose} title="" />;
  }

  const { agent, replacing } = ask;

  const submit = (event: FormEvent) => {
    event.preventDefault();

    if (replacing === null && name.trim() === "") {
      setError("Name the token.");

      return;
    }

    const at = nextSequence();

    const write =
      replacing === null
        ? meAgents.issueToken(agent.id, name.trim(), expiresAtOf(expiry))
        : meAgents.rotateToken(agent.id, replacing.id, expiresAtOf(expiry));

    void track(
      "token",
      write.then(
        ({ credential, secret }) => onIssued(agent, credential, secret, at),
        (failure: Error) => {
          const named = fieldsOf(failure);

          if (named.name !== undefined || named.expiresAt !== undefined) {
            setError([...(named.name ?? []), ...(named.expiresAt ?? [])].join(" and "));

            return;
          }

          if (turnedOff(failure)) {
            onClose();
            onTurnedOff();
          }

          toastFailure(
            replacing === null ? "Couldn't make the token" : "Couldn't rotate the token",
            failure,
          );
        },
      ),
    );
  };

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
      title={replacing === null ? `New token for ${agent.name}` : `Rotate ${replacing.name}`}
      description={
        replacing === null
          ? "You'll confirm your password, then see the token once."
          : "The new token replaces it, and the old one stops working at once."
      }
      footer={
        <>
          <Button variant="secondary" onClick={onClose}>
            Cancel
          </Button>
          <Button
            type="submit"
            form="agents-token-form"
            variant="primary"
            loading={busy("token")}
            disabled={busy("token")}
          >
            {replacing === null ? "Make token" : "Rotate token"}
          </Button>
        </>
      }
    >
      <form id="agents-token-form" className="settings-form" onSubmit={submit} noValidate>
        {replacing === null ? (
          <TextField
            label="Token name"
            value={name}
            required
            autoComplete="off"
            placeholder="Laptop"
            onChange={(event) => setName(event.target.value)}
          />
        ) : null}
        <ExpiryField value={expiry} onChange={setExpiry} />
        <FieldError message={error} />
      </form>
    </Dialog>
  );
}

/** What the edit form holds. */
interface EditForm {
  readonly name: string;
  readonly iconName: string | null;
  readonly roomIds: readonly number[];
  readonly capabilities: readonly string[];
}

/** Which parts of the form the person changed in this opening. */
interface Touched {
  readonly name: boolean;
  readonly icon: boolean;
  readonly access: boolean;
}

const UNTOUCHED: Touched = { name: false, icon: false, access: false };

/** The form for `agent` as the cache holds it. */
function formOf(agent: PersonalAgent | null, offered: readonly string[]): EditForm {
  if (agent === null) return { name: "", iconName: null, roomIds: [], capabilities: [] };

  return {
    name: agent.name,
    iconName: agent.iconName,
    roomIds: agent.roomIds,
    capabilities: agent.roomIds.length === 0 ? offered : agent.capabilities,
  };
}

/**
 * Its name, icon, rooms and abilities. One mount is one opening (the section keys it): the form is
 * seeded from the cached agent once, and only an unanswered save starts it over, from the reloaded
 * cache, and only if nothing was changed since that save was sent. What the server holds is only
 * ever the cached `agent`: a save sends just the parts the person changed that differ from it.
 */
function EditDialog({
  agent,
  list,
  writes,
  onClose,
  onTurnedOff,
}: {
  /** The cached agent, live; `null` while closed. */
  readonly agent: PersonalAgent | null;
  readonly list: PersonalAgentList;
  readonly writes: AgentWrites;
  readonly onClose: () => void;
  /** The workspace turned personal agents off since the list loaded: load it again. */
  readonly onTurnedOff: () => void;
}) {
  const [form, setForm] = useState<EditForm>(() => formOf(agent, list.capabilities));
  const [touched, setTouched] = useState<Touched>(UNTOUCHED);
  const [error, setError] = useState<string | undefined>(undefined);
  // After an unanswered save: its reload is out, or back (the form may follow the cache).
  const [recovery, setRecovery] = useState<"none" | "pending" | "arrived">("none");
  // Whether the form was changed since the last save was sent.
  const [changedSinceSave, setChangedSinceSave] = useState(false);
  const { busy, track } = useBusy();

  useEffect(() => {
    if (recovery !== "arrived") return;

    if (agent !== null && !changedSinceSave) {
      setForm(formOf(agent, list.capabilities));
      setTouched(UNTOUCHED);
    }

    setRecovery("none");
  }, [recovery, agent, changedSinceSave, list.capabilities]);

  if (agent === null) {
    return <Dialog open={false} onOpenChange={onClose} title="" />;
  }

  const edit = (change: Partial<EditForm>, part: keyof Touched) => {
    setForm((current) => ({ ...current, ...change }));
    setTouched((current) => ({ ...current, [part]: true }));
    setChangedSinceSave(true);
  };

  const submit = (event: FormEvent) => {
    event.preventDefault();

    const name = form.name.trim();

    if (name === "") {
      setError("Give the agent a name.");

      return;
    }

    if (form.roomIds.length > 0 && form.capabilities.length === 0) {
      setError("Choose at least one ability for its rooms.");

      return;
    }

    // Compared with the cache as it stands now: a rename that already took isn't sent again, and
    // a field the person left alone is never sent back over a newer value.
    const rename = touched.name && name !== agent.name;
    const reicon = touched.icon && form.iconName !== agent.iconName;

    if (!rename && !reicon && !touched.access) {
      onClose();

      return;
    }

    const save = async () => {
      let latest = agent;

      if (rename || reicon) {
        // `null` leaves that field as the server holds it.
        latest = await writes.rename(agent.id, {
          name: rename ? name : null,
          iconName: reicon ? form.iconName : null,
        });
      }

      if (touched.access) {
        latest = await writes.setAccess(agent.id, {
          roomIds: [...form.roomIds],
          capabilities: form.roomIds.length === 0 ? [] : [...form.capabilities],
        });
      }

      return latest;
    };

    setError(undefined);
    setChangedSinceSave(false);
    void track(
      "edit",
      save().then(
        (next) => {
          onClose();
          toast({ title: `${next.name} saved`, tone: "success" });
        },
        (failure: Error) => {
          const named = fieldsOf(failure);
          const messages = Object.values(named).flat();

          if (messages.length > 0) {
            setError(messages.join(" and "));

            return;
          }

          // It may have landed: load what the server holds before saving again.
          if (saveUnconfirmed(failure)) {
            setError(failure.message);
            setRecovery("pending");
            writes.reload().then(
              () => setRecovery("arrived"),
              () => setRecovery("none"),
            );

            return;
          }

          if (turnedOff(failure)) {
            onClose();
            onTurnedOff();
          }

          toastFailure("Couldn't save the agent", failure);
        },
      ),
    );
  };

  // While a save is out the dialog stays open (it's bounded, so not for long); while the reload
  // after an unanswered one is out, it can close but not save.
  const saving = busy("edit");

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !saving) onClose();
      }}
      dirty={saving}
      title={`Edit ${agent.name}`}
      footer={
        <>
          <Button variant="secondary" onClick={onClose} disabled={saving}>
            Cancel
          </Button>
          <Button
            type="submit"
            form="agents-edit-form"
            variant="primary"
            loading={saving}
            disabled={saving || recovery === "pending"}
          >
            Save
          </Button>
        </>
      }
    >
      <form id="agents-edit-form" className="settings-form" onSubmit={submit} noValidate>
        <IdentityFields
          name={form.name}
          iconName={form.iconName}
          onName={(name) => edit({ name }, "name")}
          onIcon={(iconName) => edit({ iconName }, "icon")}
        />
        <AccessFields
          rooms={list.rooms}
          capabilities={list.capabilities}
          roomIds={form.roomIds}
          granted={form.capabilities}
          onRooms={(roomIds) => edit({ roomIds }, "access")}
          onCapabilities={(capabilities) => edit({ capabilities }, "access")}
        />
        <FieldError message={error} />
      </form>
    </Dialog>
  );
}

/** A token just made, shown once with how its agent's client uses it. */
function SecretDialog({
  shown,
  mcpUrl,
  onClose,
}: {
  readonly shown: Shown | null;
  readonly mcpUrl: string | null;
  readonly onClose: () => void;
}) {
  return (
    <Dialog
      open={shown !== null}
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
      role="alertdialog"
      title={shown === null ? "" : `${shown.agent.name}'s new token`}
      footer={
        <Button variant="primary" onClick={onClose} data-autofocus>
          Done
        </Button>
      }
    >
      {shown === null ? null : (
        <SecretBlock
          secret={shown.secret}
          client={clientOf(shown.agent)?.value ?? "other"}
          agentName={shown.agent.name}
          mcpUrl={mcpUrl}
        />
      )}
    </Dialog>
  );
}

/** The wizard's draft kept across a Google confirmation, once; `null` when there's none. */
function keptWizardDraft(): WizardDraft | null {
  const kept = takeDraft(WIZARD_DRAFT_KEY);

  return kept === null ? null : parseDraft(kept);
}

/**
 * Settings → Agents: the person's own agents, each a card with its rooms, abilities and tokens,
 * and the wizard that connects a new one. When the workspace turns personal agents off, the agents
 * stop working until it's back on; the cards stay so tokens can still be revoked and agents removed.
 */
export function AgentsSection() {
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [wizard, setWizard] = useState<{ draft: WizardDraft; restored: boolean } | null>(null);
  // The agent being edited, by id (the dialog reads it from the cache), and which opening this is.
  const [editing, setEditing] = useState<number | null>(null);
  const [opening, setOpening] = useState(0);
  const [tokenAsk, setTokenAsk] = useState<TokenAsk | null>(null);
  const [shown, setShown] = useState<Shown | null>(null);
  const [ask, setAsk] = useState<Ask>(null);
  const { busy, track } = useBusy();
  const restoredOnce = useRef(false);

  // A load of the whole list, numbered when sent: it lands under what was written since.
  const loadList = useCallback((): Promise<void> => {
    const at = nextSequence();

    return meAgents.list().then((fresh) =>
      setLoad((current) => ({
        status: "ready",
        cache: withList(current.status === "ready" ? current.cache : null, fresh, at),
      })),
    );
  }, []);

  // The first load, a retry, and the quiet reload after a write finds personal agents turned
  // off: a failure shows only when there's no list to keep showing.
  const fetchList = useCallback(() => {
    loadList().catch((error: Error) =>
      setLoad((current) =>
        current.status === "ready" ? current : { status: "error", message: error.message },
      ),
    );
  }, [loadList]);

  useEffect(fetchList, [fetchList]);

  const list = load.status === "ready" ? load.cache.list : null;

  // A wizard left for a Google confirmation comes back where it stopped: on its last step.
  useEffect(() => {
    if (list === null || restoredOnce.current) return;

    restoredOnce.current = true;

    const draft = keptWizardDraft();

    if (draft !== null && canCreate(list)) {
      setWizard({ draft, restored: true });
      toast({ title: "Your new agent is back", description: "Create it to keep it." });
    }
  }, [list]);

  const reload = () => {
    setLoad({ status: "loading" });
    fetchList();
  };

  const replace = (change: (current: AgentCache) => AgentCache) =>
    setLoad((current) =>
      current.status === "ready" ? { status: "ready", cache: change(current.cache) } : current,
    );

  const writes: AgentWrites = {
    rename: (agentId, change) => {
      const at = nextSequence();

      return meAgents.update(agentId, change).then((agent) => {
        replace((current) => withAgent(current, agent, at));

        return agent;
      });
    },
    setAccess: (agentId, access) => {
      const at = nextSequence();

      return meAgents.setAccess(agentId, access).then((agent) => {
        replace((current) => withAgent(current, agent, at));

        return agent;
      });
    },
    reload: loadList,
  };

  const revoke = (agent: PersonalAgent, credential: PersonalAgentCredential) =>
    setAsk({
      title: `Revoke ${credential.name}?`,
      message: "Anything using this token stops reaching the agent at once.",
      label: "Revoke",
      danger: true,
      run: () => {
        const at = nextSequence();

        void track(
          `token-${credential.id}`,
          meAgents.revokeToken(agent.id, credential.id).then(
            ({ credentials }) => {
              replace((current) =>
                withTokens(current, agent.id, (held) => mergedTokens(held, credentials), at),
              );
              toast({ title: `${credential.name} was revoked`, tone: "success" });
            },
            (error: Error) => toastFailure(`Couldn't revoke ${credential.name}`, error),
          ),
        );
      },
    });

  const remove = (agent: PersonalAgent) =>
    setAsk({
      title: `Remove ${agent.name}?`,
      message: "It leaves its rooms and its tokens stop working. This can't be undone.",
      label: "Remove agent",
      danger: true,
      run: () => {
        const at = nextSequence();

        void track(
          `agent-${agent.id}`,
          // The answer's id is the agent's bot user, not the agent: drop the one asked for.
          meAgents.remove(agent.id).then(
            () => {
              replace((current) => withRemoved(current, agent.id, at));
              toast({ title: `${agent.name} was removed`, tone: "success" });
            },
            (error: Error) => toastFailure(`Couldn't remove ${agent.name}`, error),
          ),
        );
      },
    });

  const issued = (
    agent: PersonalAgent,
    credential: PersonalAgentCredential,
    secret: string,
    at: number,
  ) => {
    const replaced = tokenAsk?.replacing?.id;

    replace((current) =>
      withTokens(
        current,
        agent.id,
        (credentials) => [
          credential,
          ...credentials.map((c) => (c.id === replaced ? { ...c, state: "revoked" as const } : c)),
        ],
        at,
      ),
    );
    setTokenAsk(null);
    setShown({ agent, secret });
  };

  return (
    <SettingsPage
      title="Agents"
      description="Connect Claude Code, Codex, Cursor or another MCP client as your own agent. It can read and post only in the rooms you give it, and message only you."
    >
      {load.status === "loading" ? <PaneListSkeleton rows={2} /> : null}
      {load.status === "error" ? <PaneError message={load.message} onRetry={reload} /> : null}
      {list !== null && !list.enabled ? (
        <p className="settings-callout text-muted" role="status">
          Personal agents are turned off for this workspace.
          {list.agents.length > 0
            ? " Your agents stop working until it's turned back on. You can still revoke their tokens and remove them."
            : ""}{" "}
          An administrator can turn them on in{" "}
          <Link to="/admin/workspace" className="settings-link">
            Workspace settings
          </Link>
          .
        </p>
      ) : null}
      {list !== null && wizard !== null ? (
        <AgentWizard
          list={list}
          initial={wizard.draft}
          initialStep={wizard.restored ? "connect" : "client"}
          onCreated={({ agent }) => {
            const at = nextSequence();

            replace((current) => withCreated(current, agent, at));
          }}
          onRefused={fetchList}
          onClose={() => setWizard(null)}
        />
      ) : null}
      {list !== null && wizard === null ? (
        <>
          {list.enabled ? (
            <div className="agents-intro">
              <Button
                variant="primary"
                icon="bot"
                disabled={!canCreate(list)}
                onClick={() => setWizard({ draft: emptyDraft(list.capabilities), restored: false })}
              >
                Connect an agent
              </Button>
              {usage(list) === null ? null : (
                <span className="text-faint">
                  {usage(list)}
                  {canCreate(list) ? "" : ". Remove one to connect another."}
                </span>
              )}
            </div>
          ) : null}
          {list.agents.length === 0 ? (
            list.enabled ? (
              <p className="text-muted">
                No agents yet. Connect one to chat with it in your direct messages and the rooms you
                choose.
              </p>
            ) : null
          ) : (
            <SettingsGroup title="Your agents">
              <div className="agents-cards">
                {list.agents.map((agent) => (
                  <AgentCard
                    key={agent.id}
                    agent={agent}
                    list={list}
                    busy={busy}
                    onEdit={() => {
                      setEditing(agent.id);
                      setOpening((count) => count + 1);
                    }}
                    onRemove={() => remove(agent)}
                    onIssue={() => setTokenAsk({ agent, replacing: null })}
                    onRotate={(credential) => setTokenAsk({ agent, replacing: credential })}
                    onRevoke={(credential) => revoke(agent, credential)}
                  />
                ))}
              </div>
            </SettingsGroup>
          )}
        </>
      ) : null}
      {list === null ? null : (
        <>
          <EditDialog
            key={opening}
            agent={editing === null ? null : (list.agents.find((a) => a.id === editing) ?? null)}
            list={list}
            writes={writes}
            onClose={() => setEditing(null)}
            onTurnedOff={fetchList}
          />
          <TokenDialog
            ask={tokenAsk}
            onClose={() => setTokenAsk(null)}
            onIssued={issued}
            onTurnedOff={fetchList}
          />
          <SecretDialog shown={shown} mcpUrl={list.mcpUrl} onClose={() => setShown(null)} />
        </>
      )}
      <Confirm ask={ask} onCancel={() => setAsk(null)} />
    </SettingsPage>
  );
}
