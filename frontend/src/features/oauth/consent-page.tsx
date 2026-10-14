import { useNavigate, useParams } from "@tanstack/react-router";
import { useCallback, useEffect, useId, useRef, useState } from "react";
import type { OAuthConsent } from "../../gen/OAuthConsent.ts";
import type { PersonalAgent } from "../../gen/PersonalAgent.ts";
import { ActionError } from "../../sync/run.ts";
import { oauth } from "../../sync/settings.ts";
import { AgentAvatar } from "../../ui/agent-avatar.tsx";
import { Button } from "../../ui/button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { toast } from "../../ui/toast-store.ts";
import { pageExit } from "../auth/auth-navigation.ts";
import { stoppedAtConfirmation } from "../auth/confirmation.ts";
import { avatarFields } from "../bot-avatar/avatar-choice.ts";
import { PageFrame } from "../destinations/page-frame.tsx";
import { PaneError } from "../panes/pane-states.tsx";
import { AccessFields, IdentityFields } from "../settings/agent-parts.tsx";
import { agentIcon } from "../settings/agents-format.ts";
import { RoutePending } from "../shell/route-pending.tsx";
import {
  type AccessDiff,
  type AgentPick,
  accessDiff,
  badgeWarning,
  type ConsentForm,
  canCreateNew,
  consentProblem,
  firstPick,
  formFor,
  hiddenRoomsText,
  pickedAgent,
  type RoomChange,
  roomChangeText,
  unchanged,
} from "./oauth-format.ts";
import "../settings/settings.css";
import "./oauth.css";

type Shown =
  | { readonly kind: "loading" }
  | { readonly kind: "ready"; readonly consent: OAuthConsent }
  /** Expired, used already, or someone else's: the server answers all three alike. */
  | { readonly kind: "gone" }
  | { readonly kind: "failed"; readonly message: string }
  /** Approved or denied: on the way back to the app. */
  | { readonly kind: "leaving"; readonly to: string };

/** The route's page, started over for each request. */
export function ConsentRoute() {
  const { publicId } = useParams({ from: "/shell/oauth/consent/$publicId" });

  return <ConsentPage key={publicId} publicId={publicId} />;
}

/**
 * `/app/oauth/consent/:publicId`: an app asks to connect as one of the person's agents. They see
 * who is asking and where it sends them back, pick a new agent or one of theirs, set its rooms and
 * abilities (seeing what changes for an existing one), then approve, after confirming it's them,
 * or deny. Either way the browser goes back to the app.
 */
export function ConsentPage({ publicId }: { readonly publicId: string }) {
  const [shown, setShown] = useState<Shown>({ kind: "loading" });
  const [read, setRead] = useState(0);
  // Once per read, even when Strict Mode runs the effect twice.
  const ran = useRef(-1);
  const gone = useRef(false);

  useEffect(() => {
    gone.current = false;

    return () => {
      gone.current = true;
    };
  }, []);

  useEffect(() => {
    if (ran.current === read) return;

    ran.current = read;
    oauth.consent(publicId).then(
      (consent) => {
        if (!gone.current) setShown({ kind: "ready", consent });
      },
      (error: Error) => {
        if (gone.current) return;

        setShown(expired(error) ? { kind: "gone" } : { kind: "failed", message: error.message });
      },
    );
  }, [publicId, read]);

  useEffect(() => {
    document.title =
      shown.kind === "ready"
        ? `Connect ${shown.consent.clientName} · Smartfire`
        : "Connect an app · Smartfire";
  }, [shown]);

  const leave = useCallback((to: string) => {
    if (gone.current) return;

    setShown({ kind: "leaving", to });
    pageExit.assign(to);
  }, []);

  const lost = useCallback(() => {
    if (!gone.current) setShown({ kind: "gone" });
  }, []);

  switch (shown.kind) {
    case "loading":
      return <RoutePending />;

    case "leaving":
      return (
        <PageFrame title="Connect an app" icon="bot" back>
          <div className="oauth-page">
            <p className="text-muted" role="status">
              Taking you back to the app…
            </p>
          </div>
        </PageFrame>
      );

    case "gone":
      return <GonePage />;

    case "failed":
      return (
        <PageFrame title="Connect an app" icon="bot" back>
          <PaneError
            message={`Couldn't load this request. ${shown.message}`}
            onRetry={() => {
              setShown({ kind: "loading" });
              setRead((count) => count + 1);
            }}
          />
        </PageFrame>
      );

    case "ready":
      return (
        <ApprovalForm publicId={publicId} consent={shown.consent} onLeave={leave} onGone={lost} />
      );
  }
}

/** Whether the request is no longer there to answer (a 404). */
function expired(error: Error): boolean {
  return error instanceof ActionError && error.tag === "NotFound";
}

/** An expired, used or foreign request: say so plainly, and point back to the app. */
function GonePage() {
  const navigate = useNavigate();

  return (
    <PageFrame title="Connect an app" icon="bot" back>
      <div className="oauth-page">
        <div className="oauth-notice" role="alert">
          <h2 className="text-title">This request has ended</h2>
          <p className="text-muted">
            It expired, was already answered, or was started by someone else. Requests last 10
            minutes. Go back to the app and connect again to get a new one.
          </p>
        </div>
        <div className="oauth-actions">
          <Button variant="primary" onClick={() => void navigate({ to: "/" })}>
            Go home
          </Button>
        </div>
      </div>
    </PageFrame>
  );
}

/** Who is asking, where they'll send the person, and how much Smartfire knows about them. */
function ClientHeader({ consent }: { readonly consent: OAuthConsent }) {
  return (
    <header className="oauth-client">
      <h2 className="text-title">{consent.clientName} wants to connect as one of your agents</h2>
      <p className="text-muted">
        After you answer, you go back to{" "}
        <strong className="oauth-host">{consent.redirectHostname}</strong>.
      </p>
      <p className="oauth-warning" data-badge={consent.badge} role="note">
        <Icon name="circle-alert" size={16} />
        <span>{badgeWarning(consent)}</span>
      </p>
    </header>
  );
}

/** The agent to connect as: one of the person's, or a new one when they may add one. */
function AgentPicker({
  consent,
  pick,
  onPick,
}: {
  readonly consent: OAuthConsent;
  readonly pick: AgentPick;
  readonly onPick: (pick: AgentPick) => void;
}) {
  const group = useId();
  const allowNew = canCreateNew(consent);

  return (
    <fieldset className="agents-clients">
      <legend className="settings-label">Connect as</legend>
      {consent.agents.map((agent) => (
        <label key={agent.id} className="agents-client">
          <input
            type="radio"
            name={group}
            checked={pick.kind === "existing" && pick.id === agent.id}
            onChange={() => onPick({ kind: "existing", id: agent.id })}
          />
          <AgentAvatar
            seed={`${agent.userId}`}
            name={agent.name}
            spec={agent.botAvatar}
            icon={agentIcon(agent)}
            picture={agent.avatarUrl}
            size={28}
            decorative
          />
          <span className="agents-client-text">
            <span className="agents-client-label">{agent.name}</span>
            {agent.id === consent.selectedAgentId ? (
              <span className="text-faint">This app connected as it last time</span>
            ) : agent.suspended ? (
              <span className="text-faint">Suspended</span>
            ) : null}
          </span>
        </label>
      ))}
      {allowNew ? (
        <label className="agents-client">
          <input
            type="radio"
            name={group}
            checked={pick.kind === "new"}
            onChange={() => onPick({ kind: "new" })}
          />
          <span className="agents-client-text">
            <span className="agents-client-label">A new agent</span>
            <span className="text-faint">Name it and choose its rooms below</span>
          </span>
        </label>
      ) : consent.enabled ? (
        <p className="settings-hint text-faint">
          You have as many agents as this workspace allows ({consent.limit}), so connect as one of
          them.
        </p>
      ) : null}
    </fieldset>
  );
}

/** One line of the diff: a label and the names, or nothing when there are none. */
/** How a room's change line starts. */
const CHANGE_LABELS: Readonly<Record<RoomChange["change"], string>> = {
  joins: "Joins",
  leaves: "Leaves",
  abilities: "In",
};

/** One room's change: what happens to the agent there. */
function DiffLine({ change }: { readonly change: RoomChange }) {
  return (
    <li>
      <span className="oauth-diff-label">{CHANGE_LABELS[change.change]}</span> {change.room}:{" "}
      {roomChangeText(change)}
    </li>
  );
}

/** What approving changes for an existing agent, and that it reaches its other connections too. */
function DiffSummary({
  agent,
  diff,
  everywhere,
}: {
  readonly agent: PersonalAgent;
  readonly diff: AccessDiff;
  readonly everywhere: boolean;
}) {
  return (
    <section className="oauth-diff" aria-label={`Changes to ${agent.name}'s access`}>
      <h3 className="settings-label">Changes to {agent.name}'s access</h3>
      {unchanged(diff) ? (
        <p className="text-muted">No change: it keeps the rooms and abilities it has.</p>
      ) : (
        <ul className="oauth-diff-list">
          {diff.rooms.map((change) => (
            <DiffLine key={change.roomId} change={change} />
          ))}
          {diff.hiddenLeaving === 0 ? null : (
            <li>
              <span className="oauth-diff-label">Leaves</span> {hiddenRoomsText(diff.hiddenLeaving)}
            </li>
          )}
        </ul>
      )}
      {everywhere && agent.credentials.some((credential) => credential.state === "active") ? (
        <p className="settings-hint text-faint">
          This is {agent.name}'s access everywhere, for its tokens and other connections too.
        </p>
      ) : null}
    </section>
  );
}

/** The approval form itself, once the request has loaded. */
function ApprovalForm({
  publicId,
  consent,
  onLeave,
  onGone,
}: {
  readonly publicId: string;
  readonly consent: OAuthConsent;
  readonly onLeave: (to: string) => void;
  readonly onGone: () => void;
}) {
  const [form, setForm] = useState<ConsentForm>(() => formFor(consent, firstPick(consent)));
  const [busy, setBusy] = useState<"approve" | "deny" | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [nameError, setNameError] = useState<string | undefined>(undefined);
  // The name typed for a new agent, kept while an existing one is picked.
  const newName = useRef("");
  const agent = pickedAgent(consent, form.pick);

  const pick = (next: AgentPick) => {
    if (form.pick.kind === "new") newName.current = form.name;
    setForm(formFor(consent, next, newName.current));
    setProblem(null);
    setNameError(undefined);
  };

  const failed = (error: Error, doing: string) => {
    setBusy(null);

    if (stoppedAtConfirmation(error)) return;

    if (expired(error)) {
      onGone();

      return;
    }

    if (error instanceof ActionError && error.tag === "Validation") {
      const name = error.fields.name?.[0];

      if (name !== undefined) setNameError(`Name ${name}.`);
      setProblem(error.fields.base?.[0] ?? (name === undefined ? error.message : null));

      return;
    }

    if (error instanceof ActionError && error.tag === "Forbidden") {
      setProblem("Personal agents are turned off in this workspace.");

      return;
    }

    toast({ title: `Couldn't ${doing}`, description: error.message, tone: "danger" });
  };

  const approve = () => {
    const blocked = consentProblem(consent, form);

    if (blocked !== null) {
      setProblem(blocked);

      return;
    }

    setProblem(null);
    setNameError(undefined);
    setBusy("approve");
    avatarFields(form.pick.kind === "new" ? form.avatar : { kind: "keep" }, null)
      .then((picture) =>
        oauth.approve(publicId, {
          agentId: form.pick.kind === "existing" ? form.pick.id : null,
          name: form.pick.kind === "new" ? form.name.trim() : null,
          iconName: null,
          ...picture,
          roomIds: [...form.roomIds],
          capabilities: form.roomIds.length === 0 ? [] : [...form.capabilities],
        }),
      )
      .then(
        ({ redirectTo }) => onLeave(redirectTo),
        (error: Error) => failed(error, "approve"),
      );
  };

  const deny = () => {
    setBusy("deny");
    oauth.deny(publicId).then(
      ({ redirectTo }) => onLeave(redirectTo),
      (error: Error) => failed(error, "deny"),
    );
  };

  return (
    <PageFrame title={`Connect ${consent.clientName}`} icon="bot" back>
      <form
        className="oauth-page"
        noValidate
        onSubmit={(event) => {
          event.preventDefault();
          approve();
        }}
      >
        <ClientHeader consent={consent} />
        {consent.enabled ? (
          <>
            <AgentPicker consent={consent} pick={form.pick} onPick={pick} />
            {form.pick.kind === "new" ? (
              <IdentityFields
                name={form.name}
                avatar={form.avatar}
                error={nameError}
                onName={(name) => setForm((current) => ({ ...current, name }))}
                onAvatar={(avatar) => setForm((current) => ({ ...current, avatar }))}
              />
            ) : null}
            <AccessFields
              rooms={consent.rooms}
              capabilities={consent.capabilities}
              roomIds={form.roomIds}
              granted={form.capabilities}
              onRooms={(roomIds) => setForm((current) => ({ ...current, roomIds }))}
              onCapabilities={(capabilities) =>
                setForm((current) => ({ ...current, capabilities }))
              }
            />
            {agent === null ? null : (
              <DiffSummary
                agent={agent}
                diff={accessDiff(consent, agent, form)}
                everywhere={consent.accessAppliesToAllConnections}
              />
            )}
          </>
        ) : (
          <p className="oauth-notice text-muted" role="alert">
            Personal agents are turned off in this workspace, so nothing can connect. Deny to tell
            the app.
          </p>
        )}
        {problem === null ? null : (
          <p className="settings-error" role="alert">
            {problem}
          </p>
        )}
        <div className="oauth-actions">
          {consent.enabled ? (
            <Button
              type="submit"
              variant="primary"
              loading={busy === "approve"}
              disabled={busy !== null}
            >
              Approve
            </Button>
          ) : null}
          <Button loading={busy === "deny"} disabled={busy !== null} onClick={deny}>
            Deny
          </Button>
        </div>
        <p className="settings-hint text-faint">
          You can disconnect it any time in Settings → Agents.
        </p>
      </form>
    </PageFrame>
  );
}
