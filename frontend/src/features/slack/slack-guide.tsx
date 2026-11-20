import { Link } from "@tanstack/react-router";
import type { SlackPersonal } from "../../gen/SlackPersonal.ts";
import type { SlackRunMode } from "../../gen/SlackRunMode.ts";
import type { SlackRunStatus } from "../../gen/SlackRunStatus.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import "./slack.css";

export type GuideStepNumber = 1 | 2 | 3 | 4;

export interface ActiveGuideRun {
  readonly id: number;
  readonly mode: SlackRunMode;
  readonly status: SlackRunStatus;
}

export interface PersonalGuideProgress {
  readonly activeStep: GuideStepNumber | "done";
  readonly activeRun: ActiveGuideRun | null;
  readonly latestCompletedPreviewId: number | null;
  readonly hasCompletedImport: boolean;
}

export interface GuideStep {
  readonly number: GuideStepNumber;
  readonly title: string;
  readonly body: string;
}

export interface GuideFact {
  readonly title: string;
  readonly body: string;
}

/** Plain-English steps for bringing personal Slack history into Smartfire. */
export const PERSONAL_IMPORT_STEPS: readonly GuideStep[] = [
  {
    number: 1,
    title: "Connect your Slack account",
    body: "Click Connect Slack, confirm it's you if asked, and click Allow in Slack for your workspace.",
  },
  {
    number: 2,
    title: "Run a safe preview",
    body: "Click Start preview to list your DMs, group DMs, notes to self, and private channels. Nothing is copied yet.",
  },
  {
    number: 3,
    title: "Pick what to bring over",
    body: "Open the finished preview, uncheck any conversation you want to leave behind, and click Import checked.",
  },
  {
    number: 4,
    title: "Finish up in the background",
    body: "Your history copies over quietly. Once it's done, you can disconnect Slack anytime — your messages stay.",
  },
];

/** Plain-English reassurances answering common questions before importing. */
export const PERSONAL_IMPORT_FACTS: readonly GuideFact[] = [
  {
    title: "Your privacy",
    body: "New DMs, group DMs, and private channels are only visible to their Slack participants, and messages to yourself go into your personal notes. A conversation already mapped to a Smartfire room keeps that room's access.",
  },
  {
    title: "No duplicates",
    body: "If a coworker already imported a conversation you share, Smartfire recognizes it and skips duplicate messages automatically.",
  },
  {
    title: "No notification spam",
    body: "Imported history arrives quietly with its original dates and never triggers unread badges or alerts.",
  },
  {
    title: "Messages, replies, reactions & pins",
    body: "Files stay in Slack. When Slack provides a link, it appears in the imported message. Public channels are moved over separately by an admin.",
  },
];

function isActiveStatus(status: SlackRunStatus): boolean {
  return status === "queued" || status === "running" || status === "undoing";
}

/** Computes where a person currently is in the personal Slack import flow. */
export function personalGuideProgress(page: SlackPersonal): PersonalGuideProgress {
  const connected = page.connection.state === "connected";

  const activeRow = page.runs.find((run) => isActiveStatus(run.status)) ?? null;

  const activeRun: ActiveGuideRun | null =
    activeRow === null
      ? null
      : { id: activeRow.id, mode: activeRow.mode, status: activeRow.status };

  const completedImport =
    page.runs.find((run) => run.mode === "import" && run.status === "completed") ?? null;

  const hasCompletedImport = completedImport !== null;

  const completedPreview =
    page.runs.find((run) => run.mode === "dry_run" && run.status === "completed") ?? null;

  const latestCompletedPreviewId = completedPreview?.id ?? null;

  const hasNewerCompletedPreview =
    completedPreview !== null &&
    (completedImport === null || completedPreview.id > completedImport.id);

  let activeStep: GuideStepNumber | "done";

  if (activeRun !== null) {
    activeStep = activeRun.mode === "dry_run" && activeRun.status !== "undoing" ? 2 : 4;
  } else if (connected && hasNewerCompletedPreview) {
    activeStep = 3;
  } else if (hasCompletedImport) {
    activeStep = "done";
  } else if (!connected) {
    activeStep = 1;
  } else {
    activeStep = 2;
  }

  return {
    activeStep,
    activeRun,
    latestCompletedPreviewId,
    hasCompletedImport,
  };
}

/** Whether to nudge this person with the in-app Slack import prompt. */
export function shouldOfferSlackPrompt(page: SlackPersonal): boolean {
  if (!page.teamKnown) {
    return false;
  }

  const progress = personalGuideProgress(page);

  return !progress.hasCompletedImport && progress.activeRun === null;
}

type StepState = "done" | "current" | "upcoming";

function stepState(step: GuideStepNumber, active: GuideStepNumber | "done"): StepState {
  if (active === "done" || step < active) {
    return "done";
  }

  return step === active ? "current" : "upcoming";
}

function activeRunBody(run: ActiveGuideRun): string {
  if (run.status === "undoing") {
    return "Your undo is running in the background. Wait for it to finish before starting a new preview or import.";
  }

  if (run.mode === "dry_run") {
    return "Your preview is scanning your Slack conversations. Once it finishes, you'll pick which ones to bring over.";
  }

  return "Your selected conversations are copying over quietly in the background. You can leave this page and check back anytime.";
}

function activeRunLinkLabel(run: ActiveGuideRun): string {
  if (run.status === "undoing") {
    return `Undo #${run.id} is in progress — wait for it to finish`;
  }

  if (run.mode === "dry_run") {
    return `Preview #${run.id} is in progress — view status`;
  }

  return `Import #${run.id} is in progress — view status`;
}

function stepActionLink(
  step: GuideStepNumber,
  state: StepState,
  progress: PersonalGuideProgress | undefined,
  showPreviewLink: boolean,
): { readonly runId: number; readonly label: string } | null {
  if (!showPreviewLink || state !== "current" || progress === undefined) {
    return null;
  }

  if (progress.activeRun !== null) {
    return { runId: progress.activeRun.id, label: activeRunLinkLabel(progress.activeRun) };
  }

  if (step === 3 && progress.latestCompletedPreviewId !== null) {
    return {
      runId: progress.latestCompletedPreviewId,
      label: `Review preview #${progress.latestCompletedPreviewId} and pick conversations`,
    };
  }

  return null;
}

/** Ordered step-by-step walkthrough, optionally highlighting live progress on the import page. */
export function SlackImportGuideList({
  progress,
  showPreviewLink = false,
}: {
  readonly progress?: PersonalGuideProgress | undefined;
  readonly showPreviewLink?: boolean | undefined;
}) {
  return (
    <ol className="slack-guide-steps" aria-label="Steps to import your Slack history">
      {PERSONAL_IMPORT_STEPS.map((step) => {
        const state =
          progress === undefined ? "upcoming" : stepState(step.number, progress.activeStep);

        const action = stepActionLink(step.number, state, progress, showPreviewLink);

        const body =
          state === "current" && progress?.activeRun != null
            ? activeRunBody(progress.activeRun)
            : step.body;

        return (
          <li
            key={step.number}
            className="slack-guide-step"
            data-state={state}
            aria-current={state === "current" ? "step" : undefined}
          >
            <span className="slack-guide-badge" aria-hidden="true">
              {state === "done" ? <Icon name="check" size={14} /> : step.number}
            </span>
            <div className="slack-guide-copy">
              <p className="slack-guide-title">{step.title}</p>
              <p className="slack-guide-body text-muted">{body}</p>
              {action === null ? null : (
                <p className="slack-guide-action">
                  <Link
                    to="/settings/slack/$runId"
                    params={{ runId: `${action.runId}` }}
                    className="settings-classic-link"
                  >
                    {action.label}
                    <Icon name="chevron-right" size={14} />
                  </Link>
                </p>
              )}
            </div>
          </li>
        );
      })}
    </ol>
  );
}

/** Concise plain-English answers to what gets imported and how privacy works. */
export function SlackImportFacts() {
  return (
    <ul className="slack-guide-facts">
      {PERSONAL_IMPORT_FACTS.map((fact) => (
        <li key={fact.title} className="slack-guide-fact">
          <strong>{fact.title}:</strong> <span className="text-muted">{fact.body}</span>
        </li>
      ))}
    </ul>
  );
}
