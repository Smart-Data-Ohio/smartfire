import { type FormEvent, useRef, useState } from "react";
import type { PersonalAgentCreated } from "../../gen/PersonalAgentCreated.ts";
import type { PersonalAgentList } from "../../gen/PersonalAgentList.ts";
import { meAgents } from "../../sync/settings.ts";
import { Button } from "../../ui/button.tsx";
import { keepDraft } from "../admin/admin-parts.tsx";
import { leftToConfirm } from "../auth/confirmation.ts";
import { AccessFields, ExpiryField, IdentityFields, SecretBlock } from "./agent-parts.tsx";
import {
  clientChoices,
  expiresAtOf,
  STEP_LABELS,
  STEPS,
  type Step,
  serializeDraft,
  stepProblem,
  type WizardDraft,
  withClient,
} from "./agents-format.ts";
import { FieldError, fieldsOf, toastFailure, useBusy } from "./settings-parts.tsx";

/** Where the wizard waits while a Google confirmation leaves the page (never the token). */
export const WIZARD_DRAFT_KEY = "smartfire.draft.me-agent";

/** The step a refused field belongs to. */
const FIELD_STEP = new Map<string, Step>([
  ["name", "identity"],
  ["iconName", "identity"],
  ["roomIds", "access"],
  ["capabilities", "access"],
]);

/** The steps as a list, the current one marked. */
function StepList({ current }: { readonly current: Step }) {
  const at = STEPS.indexOf(current);

  return (
    <ol className="agents-steps" aria-label="Steps">
      {STEPS.map((step, index) => (
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
      {clientChoices(list.oauthAvailable).map((choice) => (
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
    </fieldset>
  );
}

/**
 * Connect an agent: the client it runs in, its name and icon, the rooms and abilities it gets,
 * then its token (after the password confirmation), shown once with that client's setup.
 */
export function AgentWizard({
  list,
  initial,
  initialStep = "client",
  onCreated,
  onClose,
}: {
  readonly list: PersonalAgentList;
  readonly initial: WizardDraft;
  readonly initialStep?: Step;
  readonly onCreated: (created: PersonalAgentCreated) => void;
  readonly onClose: () => void;
}) {
  const [step, setStep] = useState<Step>(initialStep);
  const [draft, setDraft] = useState<WizardDraft>(initial);
  const [problem, setProblem] = useState<string | null>(null);
  const [fields, setFields] = useState<Readonly<Record<string, readonly string[]>>>({});
  const [created, setCreated] = useState<PersonalAgentCreated | null>(null);
  const { busy, track } = useBusy();
  const opened = useRef(false);

  // Each step's heading mounts anew (it's keyed by the step) and takes the focus, except the
  // first one's: the button that opened the wizard keeps it.
  const heading = (element: HTMLHeadingElement | null) => {
    if (element === null) return;

    if (opened.current) element.focus();
    opened.current = true;
  };

  const at = STEPS.indexOf(step);
  const edit = (change: Partial<WizardDraft>) => setDraft((current) => ({ ...current, ...change }));

  const go = (next: Step) => {
    setProblem(null);
    setStep(next);
  };

  const create = () => {
    setFields({});
    void track(
      "create",
      meAgents
        .create({
          name: draft.name.trim(),
          iconName: draft.iconName,
          provider: clientChoices(true).find((c) => c.value === draft.client)?.provider ?? null,
          runtime: draft.client,
          roomIds: [...draft.roomIds],
          capabilities: draft.roomIds.length === 0 ? [] : [...draft.capabilities],
          tokenName: null,
          expiresAt: expiresAtOf(draft.expiry),
        })
        .then(
          (answer) => {
            setCreated(answer);
            onCreated(answer);
          },
          (error: Error) => {
            const named = fieldsOf(error);
            const first = Object.keys(named).find((key) => FIELD_STEP.has(key));

            if (first !== undefined) {
              setFields(named);
              setStep(FIELD_STEP.get(first) ?? "identity");

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

    const blocked = stepProblem(step, draft);

    if (blocked !== null) {
      setProblem(blocked);

      return;
    }

    const following = STEPS[at + 1];

    if (following === undefined) {
      create();
    } else {
      go(following);
    }
  };

  const fieldError = (key: string) => {
    const messages = fields[key];

    return messages === undefined ? undefined : messages.join(" and ");
  };

  if (created !== null) {
    return (
      <section className="agents-wizard" aria-labelledby="agents-wizard-title">
        <StepList current="connect" />
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

  const titles: Readonly<Record<Step, string>> = {
    client: "Connect an agent",
    identity: "Name your agent",
    access: "Choose what it can reach",
    connect: "Create its token",
  };

  return (
    <section className="agents-wizard" aria-labelledby="agents-wizard-title">
      <StepList current={step} />
      <h2 key={step} id="agents-wizard-title" ref={heading} tabIndex={-1} className="text-title">
        {titles[step]}
      </h2>
      <form className="settings-form" onSubmit={next} noValidate>
        {step === "client" ? <ClientStep list={list} draft={draft} onPick={setDraft} /> : null}
        {step === "identity" ? (
          <IdentityFields
            name={draft.name}
            iconName={draft.iconName}
            error={fieldError("name") ?? fieldError("iconName")}
            onName={(name) => edit({ name })}
            onIcon={(iconName) => edit({ iconName })}
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
        {step === "connect" ? (
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
              onClick={() => go(STEPS[at - 1] ?? "client")}
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
            {step === "connect" ? "Create agent" : "Next"}
          </Button>
        </div>
      </form>
    </section>
  );
}
