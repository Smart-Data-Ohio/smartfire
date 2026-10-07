import { useNavigate, useParams, useSearch } from "@tanstack/react-router";
import type { AgentApproval } from "../../gen/AgentApproval.ts";
import type { ApprovalDecision } from "../../gen/ApprovalDecision.ts";
import { profileOf } from "../../store/agents.ts";
import type { ApprovalFilter } from "../../store/approvals.ts";
import { useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { Tabs } from "../../ui/tabs.tsx";
import { toast } from "../../ui/toast-store.ts";
import { useListMotion } from "../destinations/list-motion.ts";
import { useAnnouncer } from "../destinations/live-region.tsx";
import { PagedList } from "../destinations/paged-list.tsx";
import { PaneEmpty } from "../panes/pane-states.tsx";
import { UNKNOWN_NAME } from "../people/people.ts";
import { useNow } from "../threads/use-now.ts";
import { useAgentApprovals } from "./agent-hooks.ts";
import { ApprovalCard } from "./approval-card.tsx";
import {
  APPROVAL_FILTERS,
  approvalFilterOf,
  approvalsEmptyText,
  beamedApprovalId,
  decisionAnnouncement,
} from "./approval-format.ts";

/** What a refused decision's toast says. */
function failureTitle(decision: ApprovalDecision): string {
  return decision === "approved" ? "Couldn't approve it" : "Couldn't deny it";
}

interface AgentApprovalsProps {
  readonly agentId: number;
  readonly filter: ApprovalFilter;
  readonly onFilterChange: (filter: ApprovalFilter) => void;
}

/**
 * An agent's approval requests, newest first, in a status filter: pending ones carry Approve and
 * Deny (the decision shows at once and comes back if the server refuses it), decided ones say who
 * decided and when. Decisions made elsewhere arrive live (`approval.updated`).
 */
export function AgentApprovals({ agentId, filter, onFilterChange }: AgentApprovalsProps) {
  const now = useNow();
  const view = useAgentApprovals(agentId, filter);
  const { announce, region } = useAnnouncer();

  const agentName = useStore((state) => {
    const userId = profileOf(state, agentId).profile?.agent.userId;

    return userId === undefined ? UNKNOWN_NAME : (state.users[userId]?.name ?? UNKNOWN_NAME);
  });

  const rows = useListMotion(
    view.status === "ready" ? view.rows : null,
    (approval) => approval.id,
    `${agentId}:${filter}`,
  );

  const beamed = beamedApprovalId(view.rows, now);

  const decide = (approval: AgentApproval, decision: ApprovalDecision, note: string | null) => {
    announce(decisionAnnouncement(decision, approval.summary));
    actions.approvals.decide(approval.id, decision, note).catch((error: Error) => {
      announce(`${failureTitle(decision)}: ${approval.summary}`);
      toast({ title: failureTitle(decision), description: error.message, tone: "danger" });
    });
  };

  return (
    <>
      <div className="agent-filters">
        <Tabs
          label="Status"
          items={APPROVAL_FILTERS}
          value={filter}
          onValueChange={(value) => onFilterChange(approvalFilterOf(value))}
        />
      </div>
      <PagedList
        state={view}
        label={`${agentName}'s approval requests`}
        errorText="These approval requests couldn't be loaded."
        isEmpty={rows.length === 0}
        empty={
          <PaneEmpty
            icon="shield"
            title={filter === "all" ? "No approval requests" : "None here"}
            text={approvalsEmptyText(filter, agentName)}
          />
        }
      >
        {rows.map((row) => (
          <ApprovalCard
            key={row.key}
            approval={row.value}
            now={now}
            motion={row.motion}
            beamed={row.key === beamed}
            onDecide={decide}
          />
        ))}
      </PagedList>
      {region}
    </>
  );
}

/** `/app/agents/$agentId/approvals?status=`: the approvals tab, its filter kept in the URL. */
export function AgentApprovalsRoute() {
  const { agentId } = useParams({ from: "/shell/agents/$agentId/approvals" });
  const search = useSearch({ from: "/shell/agents/$agentId/approvals" });
  const navigate = useNavigate();

  return (
    <AgentApprovals
      agentId={agentId}
      filter={search.status ?? "all"}
      onFilterChange={(filter) =>
        void navigate({
          to: "/agents/$agentId/approvals",
          params: { agentId },
          search: filter === "all" ? {} : { status: filter },
          replace: true,
        })
      }
    />
  );
}
