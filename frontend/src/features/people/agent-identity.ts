/**
 * How a bot or agent reads beside its name: the badge's words, its kind and its status, all from
 * `User.agent` (kept current by `agent.status`). The wire validates these literals but hands the
 * raw strings through, so an unknown kind or status falls back rather than breaking.
 */

import type { User } from "../../store/model.ts";
import type { IconName } from "../../ui/icons/icon.tsx";

/** The look of an agent's identity: what the badge, avatar and status line show. */
export type AgentTone = "idle" | "working" | "waiting" | "failed" | "suspended" | "inactive";

/** Who someone is, for the badge: a person (no badge), a bot without an agent row, or an agent. */
export type Identity =
  | { readonly kind: "person" }
  | { readonly kind: "bot"; readonly inactive: boolean }
  | {
      readonly kind: "agent";
      readonly agentId: number;
      readonly agentKind: string;
      readonly status: string;
      readonly suspended: boolean;
      readonly inactive: boolean;
    };

/** The identity of a user from the store; `undefined` (not loaded yet) reads as a person. */
export function identityOf(user: User | undefined): Identity {
  if (user === undefined || user.role !== "bot") {
    return { kind: "person" };
  }

  const inactive = user.status !== "active";

  if (user.agent === null) {
    return { kind: "bot", inactive };
  }

  return {
    kind: "agent",
    agentId: user.agent.agentId,
    agentKind: user.agent.kind,
    status: user.agent.status,
    suspended: user.agent.suspended,
    inactive,
  };
}

/** "Personal agent" or "Workspace agent"; a kind this client doesn't know is just "Agent". */
export function agentKindLabel(kind: string): string {
  switch (kind) {
    case "personal":
      return "Personal agent";
    case "workspace":
      return "Workspace agent";
    default:
      return "Agent";
  }
}

/** The kind's glyph, or `null` for a kind this client doesn't know. */
export function agentKindIcon(kind: string): IconName | null {
  switch (kind) {
    case "personal":
      return "user";
    case "workspace":
      return "briefcase";
    default:
      return null;
  }
}

/** "Idle", "Working", "Waiting" or "Failed"; an unknown status is shown as sent, capitalised. */
export function agentStatusLabel(status: string): string {
  switch (status) {
    case "idle":
      return "Idle";
    case "working":
      return "Working";
    case "waiting":
      return "Waiting";
    case "failed":
      return "Failed";
    default:
      return humanize(status);
  }
}

/** "needs_review" → "Needs review"; blank → "Unknown". */
export function humanize(raw: string): string {
  const spaced = raw.replaceAll("_", " ").trim();

  return spaced === "" ? "Unknown" : `${spaced.charAt(0).toUpperCase()}${spaced.slice(1)}`;
}

/** The status glyph; an unknown status gets the idle circle. */
export function agentStatusIcon(status: string): IconName {
  switch (status) {
    case "working":
      return "loader-circle";
    case "waiting":
      return "hourglass";
    case "failed":
      return "circle-x";
    default:
      return "circle";
  }
}

/**
 * The tone an agent's identity takes: deactivated or banned outranks suspended, which outranks
 * whatever status the agent last set (the classic badge reads "Suspended" whatever `status` says).
 * An unknown status reads as idle.
 */
export function agentTone(identity: Identity): AgentTone | null {
  if (identity.kind === "person") {
    return null;
  }

  if (identity.inactive) {
    return "inactive";
  }

  if (identity.kind === "bot") {
    return "idle";
  }

  if (identity.suspended) {
    return "suspended";
  }

  switch (identity.status) {
    case "working":
    case "waiting":
    case "failed":
      return identity.status;
    default:
      return "idle";
  }
}

/** The words for a tone's status: the status label, or "Suspended" / "Deactivated". */
export function toneLabel(tone: AgentTone, status: string): string {
  switch (tone) {
    case "suspended":
      return "Suspended";
    case "inactive":
      return "Deactivated";
    default:
      return agentStatusLabel(status);
  }
}
