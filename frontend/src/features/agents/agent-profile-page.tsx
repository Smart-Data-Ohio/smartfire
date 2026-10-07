import { Link, useNavigate, useParams } from "@tanstack/react-router";
import { type ReactNode, useState } from "react";
import type { AgentActivitySummary } from "../../gen/AgentActivitySummary.ts";
import type { AgentBudgetUsage } from "../../gen/AgentBudgetUsage.ts";
import type { AgentProfile } from "../../gen/AgentProfile.ts";
import { formatFull } from "../../lib/time.ts";
import { profileOf } from "../../store/agents.ts";
import { useStore } from "../../store/store.ts";
import { directs } from "../../sync/directs.ts";
import { Button } from "../../ui/button.tsx";
import { Skeleton, SkeletonReveal } from "../../ui/skeleton.tsx";
import { Tabs, tabId } from "../../ui/tabs.tsx";
import { toast } from "../../ui/toast-store.ts";
import { PageFrame } from "../destinations/page-frame.tsx";
import { PaneEmpty, PaneError } from "../panes/pane-states.tsx";
import { AgentBadge } from "../people/agent-badge.tsx";
import { agentStatusLabel } from "../people/agent-identity.ts";
import { UNKNOWN_NAME } from "../people/people.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import { timeAgo } from "../threads/thread-format.ts";
import { useNow } from "../threads/use-now.ts";
import {
  activitySentence,
  budgetFraction,
  budgetText,
  grantsLines,
  kindDescription,
  lastSeenText,
  providerLine,
  sinceText,
} from "./agent-format.ts";
import { useAgentProfile } from "./agent-hooks.ts";
import { useWorkingPresence } from "./working.ts";
import "../panes/panes.css";
import "./agents.css";

/** "Message": opens (or creates) the viewer's DM with the agent. */
function MessageButton({ userId, name }: { readonly userId: number; readonly name: string }) {
  const navigate = useNavigate();
  const [busy, setBusy] = useState(false);

  const open = () => {
    setBusy(true);
    directs.create([userId]).then(
      (row) => void navigate({ to: "/r/$roomId", params: { roomId: row.room.id } }),
      (error: Error) => {
        setBusy(false);
        toast({ title: `Couldn't message ${name}`, description: error.message, tone: "danger" });
      },
    );
  };

  return (
    <Button variant="secondary" size="sm" icon="send" loading={busy} onClick={open}>
      Message
    </Button>
  );
}

/** The status line under the name: what it's doing, its note, and since when. */
function StatusLine({ profile, now }: { readonly profile: AgentProfile; readonly now: number }) {
  const { agent } = profile;
  const presence = useWorkingPresence(agent.userId);
  const since = sinceText(agent.statusChangedAt, now);

  if (agent.suspended) {
    return (
      <p className="agent-hero-status" data-status="suspended">
        Suspended: it can't read, post or act until an administrator resumes it.
      </p>
    );
  }

  return (
    <p className="agent-hero-status" data-status={agent.status}>
      <span className="agent-hero-status-label">{agentStatusLabel(agent.status)}</span>
      {presence === null ? null : <span className="agent-row-presence"> · {presence}</span>}
      {agent.statusNote === null ? null : <span> · {agent.statusNote}</span>}
      {since === null ? null : <span className="agent-row-faint"> · {since}</span>}
    </p>
  );
}

function Hero({ profile, now }: { readonly profile: AgentProfile; readonly now: number }) {
  const { agent } = profile;
  const name = useStore((state) => state.users[agent.userId]?.name ?? UNKNOWN_NAME);
  const inactive = useStore((state) => state.users[agent.userId]?.status !== "active");

  const owner = useStore((state) =>
    agent.ownerId === null ? null : (state.users[agent.ownerId]?.name ?? UNKNOWN_NAME),
  );

  const provider = providerLine(profile.provider, profile.runtime);

  return (
    <section className="agent-hero" aria-label={name}>
      <UserAvatar userId={agent.userId} size={72} presence decorative />
      <div className="agent-hero-main">
        <div className="agent-hero-title">
          <h2 className="agent-hero-name">{name}</h2>
          <AgentBadge userId={agent.userId} status />
        </div>
        <p className="agent-hero-kind">{kindDescription(agent.kind, owner)}</p>
        <StatusLine profile={profile} now={now} />
        {profile.description === null ? null : (
          <p className="agent-hero-description">{profile.description}</p>
        )}
        <p className="agent-hero-facts">
          {provider === null ? null : <span>{provider}</span>}
          <span>{lastSeenText(agent.lastSeenAt, now)}</span>
          <span title={formatFull(agent.createdAt)}>Added {timeAgo(agent.createdAt, now)}</span>
        </p>
      </div>
      {inactive ? null : (
        <div className="agent-hero-actions">
          <MessageButton userId={agent.userId} name={name} />
        </div>
      )}
    </section>
  );
}

function Section({ title, children }: { readonly title: string; readonly children: ReactNode }) {
  return (
    <section className="agent-section">
      <h3 className="agent-section-title">{title}</h3>
      {children}
    </section>
  );
}

function Rooms({ profile }: { readonly profile: AgentProfile }) {
  const { rooms, hiddenRoomCount } = profile;

  if (rooms.length === 0 && hiddenRoomCount === 0) {
    return <p className="agent-section-empty">Not in any rooms.</p>;
  }

  return (
    <div className="agent-rooms">
      {rooms.map((room) => (
        <Link
          key={room.roomId}
          to="/r/$roomId"
          params={{ roomId: room.roomId }}
          className="agent-room"
          preload={false}
        >
          {room.name}
        </Link>
      ))}
      {hiddenRoomCount === 0 ? null : (
        <span className="agent-rooms-more">
          {rooms.length === 0 ? "" : "and "}
          {hiddenRoomCount} {hiddenRoomCount === 1 ? "room" : "rooms"} you're not in
        </span>
      )}
    </div>
  );
}

function Budget({ usage }: { readonly usage: AgentBudgetUsage }) {
  const fraction = budgetFraction(usage);

  const tone =
    fraction === null ? "none" : fraction >= 1 ? "full" : fraction >= 0.8 ? "high" : "ok";

  return (
    <li className="agent-budget" data-tone={tone}>
      <span className="agent-budget-text tabular">{budgetText(usage)}</span>
      {usage.limit === null ? <span className="agent-budget-uncapped">No daily cap</span> : null}
      {usage.limit === 0 ? <span className="agent-budget-uncapped">None allowed</span> : null}
      {fraction === null || usage.limit === null ? null : (
        <meter
          className="agent-budget-meter"
          min={0}
          max={usage.limit}
          low={usage.limit * 0.8}
          high={usage.limit * 0.99}
          optimum={0}
          value={Math.min(usage.used, usage.limit)}
          aria-label={budgetText(usage)}
        />
      )}
    </li>
  );
}

function Activity({ summary }: { readonly summary: AgentActivitySummary }) {
  const stats = [
    ["Delivered", summary.delivered],
    ["Acknowledged", summary.acknowledged],
    ["Posted", summary.posted],
    ["Suppressed", summary.suppressed],
  ] as const;

  return (
    <dl className="agent-stats" aria-label={activitySentence(summary)}>
      {stats.map(([label, value]) => (
        <div key={label} className="agent-stat" data-stat={label.toLowerCase()}>
          <dt>{label}</dt>
          <dd className="tabular">{value}</dd>
        </div>
      ))}
    </dl>
  );
}

/** The overview: rooms, then (for administrators and the owner) grants, usage and activity. */
function Overview({ profile }: { readonly profile: AgentProfile }) {
  return (
    <div className="agent-sections">
      <Section title="Rooms">
        <Rooms profile={profile} />
      </Section>
      {profile.grants === null ? null : (
        <Section title="Capabilities">
          <ul className="agent-grants">
            {grantsLines(profile.grants).map((line) => (
              <li key={line}>{line}</li>
            ))}
          </ul>
        </Section>
      )}
      {profile.management === null ? null : (
        <>
          <Section title="Today's usage">
            <ul className="agent-budgets">
              {profile.management.budgetUsage.map((usage) => (
                <Budget key={usage.cap} usage={usage} />
              ))}
            </ul>
          </Section>
          <Section title="Last 24 hours">
            <Activity summary={profile.management.activitySummary} />
          </Section>
        </>
      )}
      {profile.grants === null && profile.management === null ? (
        <p className="agent-section-note">
          Its capabilities, usage and activity are shown to administrators and its owner.
        </p>
      ) : null}
    </div>
  );
}

/** The profile once loaded: the header, then the overview. */
export function AgentProfileContent({
  profile,
  now,
}: {
  readonly profile: AgentProfile;
  readonly now: number;
}) {
  return (
    <div className="agent-profile">
      <Hero profile={profile} now={now} />
      <Overview profile={profile} />
    </div>
  );
}

function ProfileSkeleton() {
  return (
    <div className="agent-profile-skeleton" aria-hidden="true">
      <Skeleton width={72} height={72} radius="md" />
      <div className="agent-profile-skeleton-lines">
        <Skeleton width="38%" height={18} />
        <Skeleton width="56%" height={12} />
        <Skeleton width="44%" height={12} />
        <Skeleton width="72%" height={12} />
      </div>
    </div>
  );
}

/** The profile's sections: the overview, then (for administrators and the owner) two lists. */
export type AgentSection = "overview" | "approvals" | "events";

const SECTIONS: readonly { readonly value: AgentSection; readonly label: string }[] = [
  { value: "overview", label: "Overview" },
  { value: "approvals", label: "Approvals" },
  { value: "events", label: "Activity" },
];

/** The section tabs: each one is its own URL, so Back and links work. */
function SectionTabs({
  agentId,
  section,
  panelId,
}: {
  readonly agentId: number;
  readonly section: AgentSection;
  readonly panelId: string;
}) {
  const navigate = useNavigate();

  const go = (value: string) => {
    const params = { agentId };

    switch (value) {
      case "approvals":
        void navigate({ to: "/agents/$agentId/approvals", params, replace: true });
        break;
      case "events":
        void navigate({ to: "/agents/$agentId/events", params, replace: true });
        break;
      default:
        void navigate({ to: "/agents/$agentId", params, replace: true });
    }
  };

  return (
    <Tabs
      id={`${panelId}-sections`}
      panelId={panelId}
      label="Sections"
      items={SECTIONS}
      value={section}
      onValueChange={go}
    />
  );
}

/** The overview section of a loaded profile (`/app/agents/$agentId`). */
export function AgentOverviewRoute() {
  const { agentId } = useParams({ from: "/shell/agents/$agentId/" });
  const profile = useStore((state) => profileOf(state, agentId).profile);
  const now = useNow();

  return profile === null ? null : <AgentProfileContent profile={profile} now={now} />;
}

/**
 * `/app/agents/$agentId`: an agent's profile. The header has its avatar, badge, live status,
 * owner, description and provider, with a Message button; below are its rooms and, for
 * administrators and its owner, its grants, today's usage against its caps and its last 24 hours.
 * Administrators and the owner also get two more sections: its approval requests (to decide) and
 * its activity ledger. The sections render in `children` (the route's outlet).
 */
export function AgentProfilePage({
  agentId,
  section,
  children,
}: {
  readonly agentId: number;
  readonly section: AgentSection;
  readonly children: ReactNode;
}) {
  const entry = useAgentProfile(agentId);
  const panelId = `agent-${agentId}-panel`;

  const name = useStore((state) => {
    const userId = entry.profile?.agent.userId;

    return userId === undefined ? null : (state.users[userId]?.name ?? null);
  });

  const manages = entry.profile !== null && entry.profile.management !== null;

  const body = () => {
    if (entry.missing) {
      return (
        <div className="agents-scroll">
          <div className="agent-missing">
            <PaneEmpty
              icon="bot"
              title="No such agent"
              text="It may have been removed, or the link is wrong."
            />
            <Link to="/agents" className="agent-missing-link">
              See every agent
            </Link>
          </div>
        </div>
      );
    }

    if (entry.status === "error" && entry.profile === null) {
      return <PaneError message="This agent couldn't be loaded." onRetry={entry.reload} />;
    }

    // The overview resolves out of its skeleton; the lists bring their own skeletons.
    if (section === "overview" || entry.profile === null) {
      return (
        <div className="agents-scroll">
          <SkeletonReveal loading={entry.profile === null} skeleton={<ProfileSkeleton />}>
            {entry.profile === null || section !== "overview" ? null : children}
          </SkeletonReveal>
        </div>
      );
    }

    if (!manages) {
      return (
        <PaneEmpty
          icon="lock"
          title="Only for administrators and its owner"
          text={`${name ?? "This agent"}'s approval requests and activity are shown to administrators and its owner.`}
        />
      );
    }

    return children;
  };

  return (
    <PageFrame
      title={name ?? "Agent"}
      icon="bot"
      back
      toolbar={
        manages ? <SectionTabs agentId={agentId} section={section} panelId={panelId} /> : undefined
      }
    >
      {manages ? (
        <div
          className="page-panel"
          id={panelId}
          role="tabpanel"
          aria-labelledby={tabId(`${panelId}-sections`, section)}
        >
          {body()}
        </div>
      ) : (
        <div className="page-panel">{body()}</div>
      )}
    </PageFrame>
  );
}
