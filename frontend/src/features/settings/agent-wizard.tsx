import { type FormEvent, useEffect, useRef, useState } from "react";
import type { PersonalAgentCreated } from "../../gen/PersonalAgentCreated.ts";
import type { PersonalAgentList } from "../../gen/PersonalAgentList.ts";
import { meAgents } from "../../sync/settings.ts";
import { Button } from "../../ui/button.tsx";
import { keepDraft } from "../admin/admin-parts.tsx";
import { leftToConfirm } from "../auth/confirmation.ts";
import { avatarFields, keptUpload } from "../bot-avatar/avatar-choice.ts";
import {
  AccessFields,
  ExpiryField,
  IdentityFields,
  SecretBlock,
  SignInSetup,
} from "./agent-parts.tsx";
import { canSignIn } from "./agent-snippets.ts";
import {
  baseProblem,
  canCreate,
  clientChoices,
  connectChoices,
  connectsBySignIn,
  expiresAtOf,
  STEP_LABELS,
  type Step,
  serializeDraft,
  stepProblem,
  stepsFor,
  turnedOff,
  type WizardDraft,
  withClient,
} from "./agents-format.ts";
import { FieldError, fieldsOf, SettingsRadios, toastFailure, useBusy } from "./settings-parts.tsx";

/** Where the wizard waits while a Google confirmation leaves the page (never the token). */
export const WIZARD_DRAFT_KEY = "smartfire.draft.me-agent";

/** The step a refused field belongs to. */
const FIELD_STEP = new Map<string, Step>([
  ["name", "identity"],
  ["iconName", "identity"],
  ["avatar", "identity"],
  ["botAvatar", "identity"],
  ["roomIds", "access"],
  ["capabilities", "access"],
]);

/** The steps as a list, the current one marked. */
function StepList({ steps, current }: { readonly steps: readonly Step[]; readonly current: Step }) {
  const at = steps.indexOf(current);

  return (
    <ol className="agents-steps" aria-label="Steps">
      {steps.map((step, index) => (
        <li
          key={step}
          className="agents-step"
          data-done={index < at || undefined}
          aria-current={step === current ? "step" : undefined}
        >
          <span className="agents-step-number" aria-hidden="true">
            {index + 1}
          </span>
          <span className="agents-step-label">{STEP_LABELS[step]}</span>
        </li>
      ))}
    </ol>
  );
}

const SIGN_IN_CHOICES = [
  { value: "sign-in", label: "Sign in, no token" },
  { value: "token", label: "With a token" },
] as const;

/** The first step: where the agent will run. */
function ClientStep({
  list,
  draft,
  onPick,
}: {
  readonly list: PersonalAgentList;
  readonly draft: WizardDraft;
  readonly onPick: (draft: WizardDraft) => void;
}) {
  return (
    <fieldset className="agents-clients">
      <legend className="settings-label">Where will you use it?</legend>
      {connectChoices(list).map((choice) => (
        <label key={choice.value} className="agents-client">
          <input
            type="radio"
            name="agents-client"
            value={choice.value}
            checked={draft.client === choice.value}
            onChange={() => onPick(withClient(draft, choice))}
          />
          <span className="agents-client-text">
            <span className="agents-client-label">{choice.label}</span>
            <span className="text-faint">{choice.hint}</span>
          </span>
        </label>
      ))}
      {canCreate(list) ? null : (
        <p className="settings-hint text-faint">
          You have as many agents as this workspace allows, so only apps that sign in are listed.
          You'll connect one as an agent you already have.
        </p>
      )}
      {list.oauthAvailable && canSignIn(draft.client) && canCreate(list) ? (
        <SettingsRadios
          label="How it connects"
          value={draft.signIn ? "sign-in" : "token"}
          choices={SIGN_IN_CHOICES}
          onChange={(how) => onPick({ ...draft, signIn: how === "sign-in" })}
        />
      ) : null}
    </fieldset>
  );
}

/**
 * Connect an agent: the client it runs in, its name and icon, the rooms and abilities it gets,
 * then its token (after the password confirmation), shown once with that client's setup. A client
 * that signs in (the web apps, or Claude Code and Codex without a token, while the server has
 * OAuth) goes straight to its setup: the agent is chosen on the approval page it opens.
 */
export function AgentWizard({
  list,
  initial,
  initialStep = "client",
  onCreated,
  onRefused,
  onClose,
}: {
  readonly list: PersonalAgentList;
  readonly initial: WizardDraft;
  readonly initialStep?: Step;
  readonly onCreated: (created: PersonalAgentCreated) => void;
  /** The server refused for the workspace's sake (the limit, or agents turned off): reload the list. */
  readonly onRefused?: () => void;
  readonly onClose: () => void;
}) {
  const [step, setStep] = useState<Step>(initialStep);
  const [draft, setDraft] = useState<WizardDraft>(initial);
  const [problem, setProblem] = useState<string | null>(null);
  const [pictureUpload] = useState(keptUpload);
  const [fields, setFields] = useState<Readonly<Record<string, readonly string[]>>>({});
  const [created, setCreated] = useState<PersonalAgentCreated | null>(null);
  const { busy, track } = useBusy();
  const heading = useRef<HTMLHeadingElement>(null);
  // The step whose heading last took the focus; `null` before the first.
  const shown = useRef<string | null>(null);
  const view = created === null ? step : "created";

  // A new step's heading takes the focus, except the first one's: the button that opened the
  // wizard keeps it. Only a step change moves it, never typing (a re-render) within a step.
  useEffect(() => {
    if (shown.current !== null && shown.current !== view) heading.current?.focus();
    shown.current = view;
  }, [view]);

  // At the limit Claude Code and Codex can only sign in: a token would need a new agent.
  const how = canCreate(list) ? draft : { ...draft, signIn: true };
  const steps = stepsFor(how, list.oauthAvailable);
  const signingIn = connectsBySignIn(how, list.oauthAvailable);
  const at = steps.indexOf(step);
  const edit = (change: Partial<WizardDraft>) => setDraft((current) => ({ ...current, ...change }));

  const go = (next: Step) => {
    setProblem(null);
    setStep(next);
  };

  const create = () => {
    setFields({});
    setProblem(null);
    void track(
      "create",
      avatarFields(draft.avatar, null, pictureUpload)
        .then((picture) =>
          meAgents.create({
            name: draft.name.trim(),
            iconName: null,
            ...picture,
            provider: clientChoices(true).find((c) => c.value === draft.client)?.provider ?? null,
            runtime: draft.client,
            roomIds: [...draft.roomIds],
            capabilities: draft.roomIds.length === 0 ? [] : [...draft.capabilities],
            tokenName: null,
            expiresAt: expiresAtOf(draft.expiry),
          }),
        )
        .then(
          (answer) => {
            pictureUpload.settle(null);
            setCreated(answer);
            onCreated(answer);
          },
          (error: Error) => {
            pictureUpload.settle(error);

            const named = fieldsOf(error);
            const first = Object.keys(named).find((key) => FIELD_STEP.has(key));

            if (first !== undefined) {
              setFields(named);
              setStep(FIELD_STEP.get(first) ?? "identity");

              return;
            }

            // The limit (a form-level `base` refusal) and agents turned off (403) stop the whole
            // form, not a field: said under it, with the server's words.
            const formLevel = baseProblem(error) ?? (turnedOff(error) ? error.message : null);

            if (formLevel !== null) {
              setProblem(formLevel);
              onRefused?.();

              return;
            }

            if (leftToConfirm(error)) keepDraft(WIZARD_DRAFT_KEY, serializeDraft(draft));

            toastFailure("Couldn't create the agent", error);
          },
        ),
    );
  };

  const next = (event: FormEvent) => {
    event.preventDefault();

    // At the limit only signing in connects anything: it makes no agent.
    const blocked =
      draft.client !== null && !canCreate(list) && !signingIn
        ? "You have as many agents as this workspace allows. Choose an app that signs in."
        : stepProblem(step, draft);

    if (blocked !== null) {
      setProblem(blocked);

      return;
    }

    const following = steps[at + 1];

    if (following !== undefined) {
      go(following);
    } else if (signingIn) {
      onClose();
    } else {
      create();
    }
  };

  const fieldError = (key: string) => {
    const messages = fields[key];

    return messages === undefined ? undefined : messages.join(" and ");
  };

  if (created !== null) {
    return (
      <section className="agents-wizard" aria-labelledby="agents-wizard-title">
        <StepList steps={steps} current="connect" />
        <h2
          key="created"
          id="agents-wizard-title"
          ref={heading}
          tabIndex={-1}
          className="text-title"
        >
          Connect {created.agent.name}
        </h2>
        <SecretBlock
          secret={created.secret}
          client={draft.client}
          agentName={created.agent.name}
          mcpUrl={list.mcpUrl}
        />
        <div className="settings-actions">
          <Button variant="primary" onClick={onClose}>
            Done
          </Button>
        </div>
      </section>
    );
  }

  const chosen = clientChoices(true).find((choice) => choice.value === draft.client);

  const titles: Readonly<Record<Step, string>> = {
    client: "Connect an agent",
    identity: "Name your agent",
    access: "Choose what it can reach",
    connect: signingIn ? `Connect ${chosen?.label ?? "the app"}` : "Create its token",
  };

  return (
    <section className="agents-wizard" aria-labelledby="agents-wizard-title">
      <StepList steps={steps} current={step} />
      <h2 key={step} id="agents-wizard-title" ref={heading} tabIndex={-1} className="text-title">
        {titles[step]}
      </h2>
      <form className="settings-form" onSubmit={next} noValidate>
        {step === "client" ? <ClientStep list={list} draft={draft} onPick={setDraft} /> : null}
        {step === "identity" ? (
          <IdentityFields
            name={draft.name}
            avatar={draft.avatar}
            error={
              fieldError("name") ??
              fieldError("avatar") ??
              fieldError("botAvatar") ??
              fieldError("iconName")
            }
            onName={(name) => edit({ name })}
            onAvatar={(avatar) => edit({ avatar })}
          />
        ) : null}
        {step === "access" ? (
          <>
            <AccessFields
              rooms={list.rooms}
              capabilities={list.capabilities}
              roomIds={draft.roomIds}
              granted={draft.capabilities}
              onRooms={(roomIds) => edit({ roomIds })}
              onCapabilities={(capabilities) => edit({ capabilities })}
            />
            <FieldError message={fieldError("roomIds") ?? fieldError("capabilities")} />
          </>
        ) : null}
        {step === "connect" && signingIn && draft.client !== null ? (
          <SignInSetup client={draft.client} mcpUrl={list.mcpUrl} />
        ) : null}
        {step === "connect" && !signingIn ? (
          <>
            <ExpiryField value={draft.expiry} onChange={(expiry) => edit({ expiry })} />
            <p className="settings-hint text-faint">
              You'll confirm your password, then see the token once.
            </p>
          </>
        ) : null}
        <FieldError message={problem ?? undefined} />
        <div className="settings-actions agents-wizard-actions">
          <Button variant="ghost" onClick={onClose} disabled={busy("create")}>
            Cancel
          </Button>
          {at > 0 ? (
            <Button
              variant="secondary"
              icon="chevron-left"
              disabled={busy("create")}
              onClick={() => go(steps[at - 1] ?? "client")}
            >
              Back
            </Button>
          ) : null}
          <Button
            type="submit"
            variant="primary"
            loading={busy("create")}
            disabled={busy("create")}
            {...(step === "connect" ? {} : { trailingIcon: "chevron-right" as const })}
          >
            {step !== "connect" ? "Next" : signingIn ? "Done" : "Create agent"}
          </Button>
        </div>
      </form>
    </section>
  );
}
