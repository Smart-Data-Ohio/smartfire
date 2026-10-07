import { useId, useState } from "react";
import type { AgentStep } from "../../gen/AgentStep.ts";
import { Button } from "../../ui/button.tsx";
import { Icon, type IconName } from "../../ui/icons/icon.tsx";
import { humanize } from "../people/agent-identity.ts";
import "./agents.css";

/** A step's glyph; an unknown status gets a dotted circle. */
function stepIcon(status: string): IconName {
  switch (status) {
    case "pending":
      return "circle";
    case "running":
      return "loader-circle";
    case "done":
      return "circle-check";
    case "failed":
      return "circle-x";
    default:
      return "circle-dot";
  }
}

/** A step's status words; an unknown status shows as sent, capitalised. */
export function stepStatusLabel(status: string): string {
  switch (status) {
    case "pending":
      return "Pending";
    case "running":
      return "Running";
    case "done":
      return "Done";
    case "failed":
      return "Failed";
    default:
      return humanize(status);
  }
}

/** "420 ms" below a second, else "4.2 s" (the classic "{ms}ms" or seconds). */
export function formatDuration(ms: number): string {
  if (ms < 1000) {
    return `${Math.round(ms)} ms`;
  }

  const seconds = ms / 1000;

  return seconds < 10 ? `${seconds.toFixed(1)} s` : `${Math.round(seconds)} s`;
}

/** What the collapsed disclosure says beside the count. */
export interface StepsSummary {
  readonly count: string;
  /** The step running now (the first, by position), if any. */
  readonly running: AgentStep | null;
  readonly failed: number;
  /** The total of the known durations, once nothing is pending or running; `null` otherwise. */
  readonly totalMs: number | null;
}

/** "3 steps", the running step, how many failed and the time taken. */
export function summarizeSteps(steps: readonly AgentStep[]): StepsSummary {
  const running = steps.find((step) => step.status === "running") ?? null;
  const failed = steps.filter((step) => step.status === "failed").length;
  const settled = steps.every((step) => step.status === "done" || step.status === "failed");
  const durations = steps.flatMap((step) => (step.durationMs === null ? [] : [step.durationMs]));

  return {
    count: `${steps.length} ${steps.length === 1 ? "step" : "steps"}`,
    running,
    failed,
    totalMs:
      settled && durations.length > 0 ? durations.reduce((sum, each) => sum + each, 0) : null,
  };
}

function StepIcon({ status, size = 14 }: { readonly status: string; readonly size?: number }) {
  return (
    <span className="step-icon" data-status={status} aria-hidden="true">
      <Icon name={stepIcon(status)} size={size} />
    </span>
  );
}

function StepRow({ step }: { readonly step: AgentStep }) {
  return (
    <li className="step" data-status={step.status}>
      <StepIcon status={step.status} />
      <div className="step-main">
        <div className="step-line">
          <span className="step-name">{step.name}</span>
          <span className="step-status">{stepStatusLabel(step.status)}</span>
          {step.durationMs === null ? null : (
            <span className="step-duration tabular">{formatDuration(step.durationMs)}</span>
          )}
        </div>
        {step.inputSummary === null && step.outputSummary === null ? null : (
          <dl className="step-io">
            {step.inputSummary === null ? null : (
              <div>
                <dt>In</dt>
                <dd>{step.inputSummary}</dd>
              </div>
            )}
            {step.outputSummary === null ? null : (
              <div>
                <dt>Out</dt>
                <dd>{step.outputSummary}</dd>
              </div>
            )}
          </dl>
        )}
      </div>
    </li>
  );
}

/**
 * The steps an agent reported for a message, under its body: a disclosure that reads "3 steps"
 * with the running step's name live (its icon turns slowly; still under reduced motion), or how
 * many failed and the time taken once done. Opening it lists every step with its status, time and
 * In/Out summaries (the transitions.dev accordion). Kept current by `agent.steps`.
 */
export function MessageSteps({ steps }: { readonly steps: readonly AgentStep[] }) {
  const id = useId();
  const [open, setOpen] = useState(false);

  if (steps.length === 0) {
    return null;
  }

  const summary = summarizeSteps(steps);
  const lead = summary.running?.status ?? (summary.failed > 0 ? "failed" : "done");

  return (
    <div className="steps t-acc" data-open={open}>
      <Button
        variant="ghost"
        size="sm"
        className="steps-toggle"
        aria-expanded={open}
        aria-controls={`${id}-steps`}
        onClick={() => setOpen(!open)}
      >
        <span className="steps-summary">
          <StepIcon status={lead} />
          <span className="steps-count">{summary.count}</span>
          {summary.running === null ? null : (
            <span className="steps-current">{summary.running.name}</span>
          )}
          {summary.running === null && summary.failed > 0 ? (
            <span className="steps-failed">{summary.failed} failed</span>
          ) : null}
          {summary.totalMs === null ? null : (
            <span className="steps-total tabular">{formatDuration(summary.totalMs)}</span>
          )}
          <span className="t-acc-chevron steps-chevron">
            <Icon name="chevron-down" size={12} />
          </span>
        </span>
      </Button>
      <div id={`${id}-steps`} className="t-acc-panel" inert={!open}>
        <div className="t-acc-panel-inner">
          <ol className="steps-list" aria-label="Steps">
            {steps.map((step) => (
              <StepRow key={step.id} step={step} />
            ))}
          </ol>
        </div>
      </div>
    </div>
  );
}
