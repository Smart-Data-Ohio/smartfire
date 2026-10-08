import { Link } from "@tanstack/react-router";
import { useId, useState } from "react";
import type { AgentDirectoryRow } from "../../gen/AgentDirectoryRow.ts";
import type { User } from "../../store/model.ts";
import { useStore } from "../../store/store.ts";
import { SkeletonReveal } from "../../ui/skeleton.tsx";
import { Tabs, tabId } from "../../ui/tabs.tsx";
import { PageFrame } from "../destinations/page-frame.tsx";
import { PaneEmpty, PaneError, PaneListSkeleton, PaneSearch } from "../panes/pane-states.tsx";
import { AgentBadge } from "../people/agent-badge.tsx";
import { agentStatusLabel } from "../people/agent-identity.ts";
import { UNKNOWN_NAME } from "../people/people.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import { useNow } from "../threads/use-now.ts";
import { foldText, kindDescription, lastSeenText, sinceText } from "./agent-format.ts";
import { useAgentDirectory } from "./agent-hooks.ts";
import { useWorkingPresence } from "./working.ts";
import "../panes/panes.css";
import "./agents.css";

/** The directory's kind filter. */
const KIND_TABS = [
  { value: "all", label: "All" },
  { value: "personal", label: "Personal" },
  { value: "workspace", label: "Workspace" },
] as const;

type KindFilter = (typeof KIND_TABS)[number]["value"];

function isKindFilter(value: string): value is KindFilter {
  return KIND_TABS.some((tab) => tab.value === value);
}

/** The rows matching the kind tab and the search (the agent's or its owner's name). */
export function filterAgents(
  rows: readonly AgentDirectoryRow[],
  users: Readonly<Record<number, User>>,
  kind: KindFilter,
  query: string,
): readonly AgentDirectoryRow[] {
  const folded = foldText(query);

  return rows.filter((row) => {
    if (kind !== "all" && row.kind !== kind) {
      return false;
    }

    if (folded === "") {
      return true;
    }

    const owner = row.ownerId === null ? "" : (users[row.ownerId]?.name ?? "");

    return [users[row.userId]?.name ?? "", owner].some((name) => foldText(name).includes(folded));
  });
}

/** The status line: "Working · Reviewing the deploy · since 3 minutes ago". */
function StatusLine({ row, now }: { readonly row: AgentDirectoryRow; readonly now: number }) {
  const presence = useWorkingPresence(row.userId);
  const since = sinceText(row.statusChangedAt, now);

  if (row.suspended) {
    return <span className="agent-row-status">Suspended; it can't act until resumed</span>;
  }

  return (
    <span className="agent-row-status" data-status={row.status}>
      <span className="agent-row-status-label">{agentStatusLabel(row.status)}</span>
      {presence === null ? null : <span className="agent-row-presence"> · {presence}</span>}
      {presence === null && row.statusNote !== null ? <span> · {row.statusNote}</span> : null}
      {since === null ? null : <span className="agent-row-faint"> · {since}</span>}
    </span>
  );
}

function DirectoryRow({ row, now }: { readonly row: AgentDirectoryRow; readonly now: number }) {
  const name = useStore((state) => state.users[row.userId]?.name ?? UNKNOWN_NAME);

  const owner = useStore((state) =>
    row.ownerId === null ? null : (state.users[row.ownerId]?.name ?? UNKNOWN_NAME),
  );

  return (
    <li className="agent-row" data-suspended={row.suspended || undefined}>
      <Link
        to="/agents/$agentId"
        params={{ agentId: row.agentId }}
        className="agent-row-open"
        preload={false}
      >
        <UserAvatar userId={row.userId} size={36} presence decorative />
        <span className="agent-row-main">
          <span className="agent-row-title">
            <span className="agent-row-name">{name}</span>
            <AgentBadge userId={row.userId} status />
          </span>
          <span className="agent-row-meta">
            {kindDescription(row.kind, owner)}
            <span className="agent-row-faint"> · {lastSeenText(row.lastSeenAt, now)}</span>
          </span>
          <StatusLine row={row} now={now} />
        </span>
      </Link>
    </li>
  );
}

/**
 * `/app/agents`: every agent in the workspace with its kind, owner and live status, searchable by
 * its or its owner's name and filtered by kind. A row opens the agent's profile.
 */
export function AgentDirectoryPage() {
  const view = useAgentDirectory();
  const users = useStore((state) => state.users);
  const now = useNow();
  const ids = useId();
  const [kind, setKind] = useState<KindFilter>("all");
  const [query, setQuery] = useState("");
  const tabsId = `${ids}-kind`;
  const panelId = `${ids}-panel`;
  const rows = filterAgents(view.rows, users, kind, query);
  const filtered = kind !== "all" || query.trim() !== "";

  return (
    <PageFrame
      title="Agents"
      icon="bot"
      back
      meta={view.status === "ready" ? <span className="page-count">{view.rows.length}</span> : null}
      toolbar={
        <div className="agents-toolbar">
          <Tabs
            id={tabsId}
            panelId={panelId}
            label="Kind"
            items={KIND_TABS}
            value={kind}
            onValueChange={(value) => {
              if (isKindFilter(value)) {
                setKind(value);
              }
            }}
          />
          <PaneSearch value={query} onValueChange={setQuery} label="Find an agent or owner" />
        </div>
      }
    >
      <div
        role="tabpanel"
        id={panelId}
        aria-labelledby={tabId(tabsId, kind)}
        className="page-panel agents-scroll"
      >
        {view.status === "error" ? (
          <PaneError message="The agents couldn't be loaded." onRetry={view.reload} />
        ) : (
          <SkeletonReveal
            loading={view.status !== "ready"}
            skeleton={
              <div className="page-skeleton">
                <PaneListSkeleton rows={5} square={36} />
              </div>
            }
          >
            <p className="agents-lede">Every agent in the workspace, with live status.</p>
            {rows.length === 0 ? (
              filtered ? (
                <PaneEmpty
                  icon="search"
                  title="No agents match"
                  text="Try another name, or show every kind."
                />
              ) : (
                <PaneEmpty
                  icon="bot"
                  title="No agents yet"
                  text="Agents show up here once an administrator adds one."
                />
              )
            ) : (
              <ul className="agent-list" aria-label="Agents">
                {rows.map((row) => (
                  <DirectoryRow key={row.agentId} row={row} now={now} />
                ))}
              </ul>
            )}
          </SkeletonReveal>
        )}
      </div>
    </PageFrame>
  );
}
