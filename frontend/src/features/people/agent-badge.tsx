import type { User } from "../../store/model.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import {
  agentKindIcon,
  agentKindLabel,
  agentStatusIcon,
  agentTone,
  identityOf,
  toneLabel,
} from "./agent-identity.ts";
import { useUser } from "./people.ts";
import "./agent-badge.css";

interface AgentBadgeProps {
  readonly userId: number;
  /** Also show the agent's live status (Working, Waiting, Failed); suspension always shows. */
  readonly status?: boolean;
}

/**
 * The AGENT badge beside a bot's name, from `User.agent`: violet with its kind's glyph (personal
 * or workspace; an unknown kind gets the plain badge), "Bot" for a bot without an agent row, and
 * a "Suspended" or "Deactivated" chip when it can't act. Renders nothing for people, so every name
 * can carry it.
 */
export function AgentBadge({ userId, status = false }: AgentBadgeProps) {
  return <AgentBadgeFor user={useUser(userId)} status={status} />;
}

/** {@link AgentBadge} for a user already in hand (autocomplete rows carry their own). */
export function AgentBadgeFor({
  user,
  status = false,
}: {
  readonly user: User | undefined;
  readonly status?: boolean;
}) {
  const identity = identityOf(user);
  const tone = agentTone(identity);

  if (identity.kind === "person" || tone === null) {
    return null;
  }

  const blocked = tone === "suspended" || tone === "inactive";
  const shownStatus = identity.kind === "agent" ? identity.status : "idle";
  const showStatus = blocked || (status && identity.kind === "agent" && tone !== "idle");

  return (
    <span className="agent-badges">
      {identity.kind === "bot" ? (
        <span className="agent-badge" data-tone="bot">
          Bot
        </span>
      ) : (
        <AgentTag kind={identity.agentKind} />
      )}
      {showStatus ? (
        <span className="agent-state" data-tone={tone}>
          {blocked ? null : <Icon name={agentStatusIcon(shownStatus)} size={11} />}
          {toneLabel(tone, shownStatus)}
        </span>
      ) : null}
    </span>
  );
}

function AgentTag({ kind }: { readonly kind: string }) {
  const icon = agentKindIcon(kind);
  const label = agentKindLabel(kind);

  return (
    <span className="agent-badge" data-kind={icon === null ? undefined : kind} title={label}>
      {icon === null ? null : <Icon name={icon} size={10} />}
      Agent
      {icon === null ? null : <span className="visually-hidden"> ({label.toLowerCase()})</span>}
    </span>
  );
}
