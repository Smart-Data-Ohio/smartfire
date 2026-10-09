import { useNavigate, useParams, useSearch } from "@tanstack/react-router";
import { profileOf } from "../../store/agents.ts";
import type { LedgerFilter } from "../../store/ledger.ts";
import { useStore } from "../../store/store.ts";
import { Tabs } from "../../ui/tabs.tsx";
import { PagedList } from "../destinations/paged-list.tsx";
import { PaneEmpty } from "../panes/pane-states.tsx";
import { UNKNOWN_NAME } from "../people/people.ts";
import { useNow } from "../threads/use-now.ts";
import { useAgentLedger } from "./agent-hooks.ts";
import { LEDGER_FILTERS, ledgerEmptyText, ledgerFilterOf } from "./ledger-format.ts";
import { LedgerRow } from "./ledger-row.tsx";

interface AgentLedgerProps {
  readonly agentId: number;
  readonly filter: LedgerFilter;
  readonly onFilterChange: (filter: LedgerFilter) => void;
}

/**
 * An agent's event ledger, newest first, in an outcome filter: what was delivered to it, what it
 * did, and what was held back and why. Only administrators and the agent's owner may read it; for
 * anyone else it says so (with no Retry). It has no live updates, so it reloads each time it's
 * shown.
 */
export function AgentLedger({ agentId, filter, onFilterChange }: AgentLedgerProps) {
  const now = useNow();
  const view = useAgentLedger(agentId, filter);

  const agentName = useStore((state) => {
    const userId = profileOf(state, agentId).profile?.agent.userId;

    return userId === undefined ? UNKNOWN_NAME : (state.users[userId]?.name ?? UNKNOWN_NAME);
  });

  if (view.forbidden) {
    return (
      <PaneEmpty
        icon="lock"
        title="Activity is private"
        text={`Only administrators and ${agentName}'s owner can see what was delivered to it.`}
      />
    );
  }

  return (
    <>
      <div className="agent-filters">
        <Tabs
          label="Outcome"
          items={LEDGER_FILTERS}
          value={filter}
          onValueChange={(value) => onFilterChange(ledgerFilterOf(value))}
        />
      </div>
      <PagedList
        state={view}
        label={`${agentName}'s activity`}
        errorText="This agent's activity couldn't be loaded."
        isEmpty={view.rows.length === 0}
        empty={<PaneEmpty icon="inbox" title="No events" text={ledgerEmptyText(filter)} />}
      >
        {view.rows.map((event) => (
          <LedgerRow key={event.id} event={event} agentName={agentName} now={now} />
        ))}
      </PagedList>
    </>
  );
}

/** `/app/agents/$agentId/events?outcome=`: the activity tab, its filter kept in the URL. */
export function AgentLedgerRoute() {
  const { agentId } = useParams({ from: "/shell/agents/$agentId/events" });
  const search = useSearch({ from: "/shell/agents/$agentId/events" });
  const navigate = useNavigate();

  return (
    <AgentLedger
      agentId={agentId}
      filter={search.outcome ?? "all"}
      onFilterChange={(filter) =>
        void navigate({
          to: "/agents/$agentId/events",
          params: { agentId },
          search: filter === "all" ? {} : { outcome: filter },
          replace: true,
        })
      }
    />
  );
}
