/**
 * Settings → Agents' copy of what the server holds, kept in order: latest sent wins.
 *
 * Every request about an agent (a rename, an access save, a token change, a removal, a reload of
 * the whole list) takes a number from `nextSequence()` when it is sent. An answer lands only where
 * no answer to a later-sent request has landed: for an agent's profile and access as a whole, and
 * for each of its tokens one by one. So a slow reload never puts back an older name, a slow
 * revocation's list never shows a token another revocation already ended as active again, and an
 * older reload can't bring back a removed agent (it keeps its number). Tokens are never deleted,
 * only revoked, so an answer that doesn't mention one leaves it as it is.
 */
import type { PersonalAgent } from "../../gen/PersonalAgent.ts";
import type { PersonalAgentCredential } from "../../gen/PersonalAgentCredential.ts";
import type { PersonalAgentList } from "../../gen/PersonalAgentList.ts";

/** The newest request applied to one agent. */
interface Applied {
  /** Its profile and access: everything but its tokens (and its removal). */
  readonly agent: number;
  /** Each of its tokens, by token id. */
  readonly tokens: ReadonlyMap<number, number>;
}

export interface AgentCache {
  readonly list: PersonalAgentList;
  /** By agent id, removed agents included. */
  readonly applied: ReadonlyMap<number, Applied>;
  /** The newest whole-list load applied: it speaks for everything not written to since. */
  readonly listSequence: number;
}

let sequence = 0;

/** The next request's number; numbers only grow. */
export function nextSequence(): number {
  sequence += 1;

  return sequence;
}

const NOTHING: Applied = { agent: 0, tokens: new Map() };

function appliedTo(cache: AgentCache, agentId: number): Applied {
  return cache.applied.get(agentId) ?? NOTHING;
}

/** The newest request applied to agent `agentId`'s profile and access. */
function profileAt(cache: AgentCache, agentId: number): number {
  return Math.max(appliedTo(cache, agentId).agent, cache.listSequence);
}

/** An agent's tokens after an answer, and the request each now stands at. */
interface MergedTokens {
  readonly credentials: PersonalAgentCredential[];
  readonly tokens: ReadonlyMap<number, number>;
}

/**
 * `held` tokens with `incoming` ones (from the answer to request `at`) in place of any that no
 * later-answered request has changed, and the numbers they now stand at.
 */
function mergedTokens(
  cache: AgentCache,
  agentId: number,
  held: readonly PersonalAgentCredential[],
  incoming: readonly PersonalAgentCredential[],
  at: number,
): MergedTokens {
  const recorded = appliedTo(cache, agentId).tokens;
  const tokens = new Map(recorded);
  const newer = (id: number) => at > Math.max(recorded.get(id) ?? 0, cache.listSequence);
  const taken = incoming.filter((credential) => newer(credential.id));

  for (const credential of taken) tokens.set(credential.id, at);

  const kept = held.filter((h) => !taken.some((t) => t.id === h.id));

  return { credentials: [...taken, ...kept], tokens };
}

function settled(
  cache: AgentCache,
  agentId: number,
  agents: readonly PersonalAgent[],
  applied: Applied,
): AgentCache {
  const next = new Map(cache.applied);

  next.set(agentId, applied);

  return { ...cache, list: { ...cache.list, agents: [...agents] }, applied: next };
}

/** A whole-list load sent as request `at`, over what `cache` holds (if anything yet). */
export function withList(
  cache: AgentCache | null,
  fresh: PersonalAgentList,
  at: number,
): AgentCache {
  if (cache === null) return { list: fresh, applied: new Map(), listSequence: at };

  if (at <= cache.listSequence) return cache;

  const listed = new Set(fresh.agents.map((agent) => agent.id));
  const applied = new Map(cache.applied);

  const agents = fresh.agents.flatMap((agent) => {
    const held = cache.list.agents.find((a) => a.id === agent.id);
    const profileNewer = appliedTo(cache, agent.id).agent > at;

    // Removed after this load was sent.
    if (held === undefined) return profileNewer ? [] : [agent];

    const { credentials, tokens } = mergedTokens(
      cache,
      agent.id,
      held.credentials,
      agent.credentials,
      at,
    );

    applied.set(agent.id, { agent: appliedTo(cache, agent.id).agent, tokens });

    return [{ ...(profileNewer ? held : agent), credentials }];
  });

  // Connected after this load was sent, so it couldn't list them.
  const later = cache.list.agents.filter(
    (agent) => !listed.has(agent.id) && appliedTo(cache, agent.id).agent > at,
  );

  return { list: { ...fresh, agents: [...agents, ...later] }, applied, listSequence: at };
}

/** An agent as the answer to request `at` (a rename or an access save) gives it. */
export function withAgent(cache: AgentCache, agent: PersonalAgent, at: number): AgentCache {
  const held = cache.list.agents.find((a) => a.id === agent.id);

  if (held === undefined) return cache;

  const { credentials, tokens } = mergedTokens(
    cache,
    agent.id,
    held.credentials,
    agent.credentials,
    at,
  );

  const newer = at > profileAt(cache, agent.id);
  const next = { ...(newer ? agent : held), credentials };

  return settled(
    cache,
    agent.id,
    cache.list.agents.map((a) => (a.id === agent.id ? next : a)),
    { agent: newer ? at : appliedTo(cache, agent.id).agent, tokens },
  );
}

/** A new agent, connected by the request answered as `at`. */
export function withCreated(cache: AgentCache, agent: PersonalAgent, at: number): AgentCache {
  if (cache.list.agents.some((a) => a.id === agent.id)) return cache;

  return settled(cache, agent.id, [...cache.list.agents, agent], {
    agent: at,
    tokens: new Map(agent.credentials.map((credential) => [credential.id, at])),
  });
}

/** Agent `agentId` gone, by request `at`; an older load won't bring it back. */
export function withRemoved(cache: AgentCache, agentId: number, at: number): AgentCache {
  return settled(
    cache,
    agentId,
    cache.list.agents.filter((a) => a.id !== agentId),
    { ...appliedTo(cache, agentId), agent: Math.max(profileAt(cache, agentId), at) },
  );
}

/**
 * Agent `agentId`'s tokens as the answer to request `at` gives them: an issued token, a rotated
 * one with the one it replaced, or a revocation's whole list. Each lands only if no later-sent
 * request's answer has changed it.
 */
export function withTokens(
  cache: AgentCache,
  agentId: number,
  incoming: readonly PersonalAgentCredential[],
  at: number,
): AgentCache {
  const held = cache.list.agents.find((a) => a.id === agentId);

  if (held === undefined) return cache;

  const { credentials, tokens } = mergedTokens(cache, agentId, held.credentials, incoming, at);

  return settled(
    cache,
    agentId,
    cache.list.agents.map((a) => (a.id === agentId ? { ...held, credentials } : a)),
    { agent: appliedTo(cache, agentId).agent, tokens },
  );
}
