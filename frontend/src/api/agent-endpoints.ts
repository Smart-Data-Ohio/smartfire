/**
 * The S4 agent endpoints: the directory, an agent's profile, its approval requests (and deciding
 * one) and its event ledger. Each validates the reply with its pinned schema (see endpoints.ts).
 */
import { Effect } from "effect";
import type { AgentApproval } from "../gen/AgentApproval.ts";
import type { AgentApprovalPage } from "../gen/AgentApprovalPage.ts";
import type { AgentApprovalStatus } from "../gen/AgentApprovalStatus.ts";
import type { AgentDeliveryOutcome } from "../gen/AgentDeliveryOutcome.ts";
import type { AgentDirectory } from "../gen/AgentDirectory.ts";
import type { AgentLedgerPage } from "../gen/AgentLedgerPage.ts";
import type { AgentProfile } from "../gen/AgentProfile.ts";
import type { DecideApproval } from "../gen/DecideApproval.ts";
import { call, get } from "./call.ts";
import {
  AgentApprovalPage as AgentApprovalPageSchema,
  AgentApproval as AgentApprovalSchema,
  AgentDirectory as AgentDirectorySchema,
  AgentLedgerPage as AgentLedgerPageSchema,
  AgentProfile as AgentProfileSchema,
} from "./schema/agents.ts";
import { wire } from "./wire.ts";

/** `GET /agents`: every agent, active ones first, then by name; not paged. A bot gets a 403. */
export const agentDirectory = Effect.fn("api.agentDirectory")(function* () {
  return yield* call(get("/agents"), wire<AgentDirectory>(AgentDirectorySchema));
});

/**
 * `GET /agents/:agentId`: the profile. `grants` and `management` are `null` unless the viewer is an
 * administrator or the agent's owner; an unknown agent is a 404.
 */
export const agentProfile = Effect.fn("api.agentProfile")(function* (agentId: number) {
  return yield* call(get(`/agents/${agentId}`), wire<AgentProfile>(AgentProfileSchema));
});

/** A page's query: the filter when there is one, and `before` past the first page. */
function pageQuery(
  filterKey: string,
  filter: string | null,
  before: string | null,
): Readonly<Record<string, string>> {
  const query = new URLSearchParams();

  if (filter !== null) {
    query.set(filterKey, filter);
  }

  if (before !== null) {
    query.set("before", before);
  }

  return Object.fromEntries(query);
}

/**
 * `GET /agents/:agentId/approvals?status=&before=`: 50 of the agent's approval requests, newest
 * first, in one state or all (`status` `null`). Only administrators and the owner; anyone else
 * gets a 404, as in the classic app.
 */
export const agentApprovals = Effect.fn("api.agentApprovals")(function* (
  agentId: number,
  status: AgentApprovalStatus | null,
  before: string | null,
) {
  return yield* call(
    get(`/agents/${agentId}/approvals`, pageQuery("status", status, before)),
    wire<AgentApprovalPage>(AgentApprovalPageSchema),
  );
});

/**
 * `PATCH /agent_approvals/:id`: approve or deny. A 403 for an admin-only action, a 404 when the
 * viewer may no longer decide it, a 422 when it isn't pending any more.
 */
export const decideApproval = Effect.fn("api.decideApproval")(function* (
  approvalId: number,
  body: DecideApproval,
) {
  return yield* call(
    { method: "PATCH", path: `/agent_approvals/${approvalId}`, body },
    wire<AgentApproval>(AgentApprovalSchema),
  );
});

/**
 * `GET /agents/:agentId/events?outcome=&before=`: 50 ledger entries, newest first, with one
 * delivery outcome or all (`outcome` `null`). Administrators and the owner only (403 otherwise).
 */
export const agentLedger = Effect.fn("api.agentLedger")(function* (
  agentId: number,
  outcome: AgentDeliveryOutcome | null,
  before: string | null,
) {
  return yield* call(
    get(`/agents/${agentId}/events`, pageQuery("outcome", outcome, before)),
    wire<AgentLedgerPage>(AgentLedgerPageSchema),
  );
});
