/**
 * A work thread's agent steps, plainly: name, status and how long it took. Deliberately minimal
 * and self-contained; the agent helper owns the polished steps component and can swap it in here.
 */
import type { AgentStep } from "../../gen/AgentStep.ts";
import { Icon, type IconName } from "../../ui/icons/icon.tsx";

/** How a step's status shows. */
interface StepLook {
  readonly label: string;
  readonly icon: IconName;
}

/** A step's label and icon; a status this client doesn't know reads "Unknown". */
function stepStatus(status: string): StepLook {
  switch (status) {
    case "pending":
      return { label: "Pending", icon: "clock" };
    case "running":
      return { label: "Running", icon: "circle-dot" };
    case "done":
      return { label: "Done", icon: "circle-check" };
    case "failed":
      return { label: "Failed", icon: "alert" };
    default:
      return { label: "Unknown", icon: "circle-dot" };
  }
}

/** "120 ms" below a second, else "4.2 s"; `null` while unknown. */
export function stepDuration(ms: number | null): string | null {
  if (ms === null) {
    return null;
  }

  return ms < 1000 ? `${ms} ms` : `${(ms / 1000).toFixed(1)} s`;
}

/** The steps as a list, in their order; nothing when there are none. */
export function WorkSteps({ steps }: { readonly steps: readonly AgentStep[] }) {
  if (steps.length === 0) {
    return null;
  }

  return (
    <ol className="work-steps">
      {steps.map((step) => {
        const status = stepStatus(step.status);
        const duration = stepDuration(step.durationMs);

        return (
          <li key={step.id} className="work-step" data-status={step.status}>
            <Icon name={status.icon} size={14} className="work-step-icon" />
            <span className="work-step-name">{step.name}</span>
            <span className="work-step-meta">
              {status.label}
              {duration === null ? null : ` · ${duration}`}
            </span>
          </li>
        );
      })}
    </ol>
  );
}
