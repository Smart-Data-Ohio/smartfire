/**
 * Settings → Agents' copy of what the server holds, kept in order: latest sent wins.
 *
 * Every request about an agent (a rename, an access save, a token change, a reload of the whole
 * list) takes a number from `nextSequence()` when it is sent. An answer lands only if no answer to
 * a later-sent request has landed for that part of that agent: its profile and access, or its
 * tokens. So a slow reload never puts back an older name, and a slow rename never undoes what a
 * later reload found. A removed agent keeps its number, so an older reload can't bring it back.
 */
import type { PersonalAgent } from "../../gen/PersonalAgent.ts";
import type { PersonalAgentCredential } from "../../gen/PersonalAgentCredential.ts";
import type { PersonalAgentList } from "../../gen/PersonalAgentList.ts";

/** The newest request applied to each part of one agent. */
interface Applied {
  /** Its profile and access: everything but its tokens (and its removal). */
  readonly agent: number;
  readonly tokens: number;
}

export interface AgentCache {
  readonly list: PersonalAgentList;
  /** By agent id, removed agents included. */
  readonly applied: ReadonlyMap<number, Applied>;
  /** The newest whole-list load applied: it speaks for every agent not written to since. */
  readonly listSequence: number;
}

let sequence = 0;

/** The next request's number; numbers only grow. */
export function nextSequence(): number {
  sequence += 1;

  return sequence;
}

function appliedTo(cache: AgentCache, agentId: number): Applied {
  const applied = cache.applied.get(agentId);

  return {
    agent: Math.max(applied?.agent ?? 0, cache.listSequence),
    tokens: Math.max(applied?.tokens ?? 0, cache.listSequence),
  };
}

function marked(cache: AgentCache, agentId: number, change: Partial<Applied>): AgentCache {
  const applied = new Map(cache.applied);

  applied.set(agentId, { ...appliedTo(cache, agentId), ...change });

  return { ...cache, applied };
}

function replaced(cache: AgentCache, agent: PersonalAgent): AgentCache {
  return {
    ...cache,
    list: { ...cache.list, agents: cache.list.agents.map((a) => (a.id === agent.id ? agent : a)) },
  };
}

/** A whole-list load sent as request `sequence`, over what `cache` holds (if anything yet). */
export function withList(
  cache: AgentCache | null,
  fresh: PersonalAgentList,
  at: number,
): AgentCache {
  if (cache === null) return { list: fresh, applied: new Map(), listSequence: at };

  if (at <= cache.listSequence) return cache;

  const listed = new Set(fresh.agents.map((agent) => agent.id));

  const agents = fresh.agents.flatMap((agent) => {
    const recorded = cache.applied.get(agent.id);
    const held = cache.list.agents.find((a) => a.id === agent.id);

    if (recorded === undefined || (recorded.agent < at && recorded.tokens < at)) return [agent];

    // Written to (or removed) after this load was sent: its newer parts stay.
    if (held === undefined) return recorded.agent > at ? [] : [agent];

    const profile = recorded.agent > at ? held : agent;

    return [
      { ...profile, credentials: recorded.tokens > at ? held.credentials : agent.credentials },
    ];
  });

  // Connected after this load was sent, so it couldn't list them.
  const later = cache.list.agents.filter(
    (agent) => !listed.has(agent.id) && (cache.applied.get(agent.id)?.agent ?? 0) > at,
  );

  return {
    list: { ...fresh, agents: [...agents, ...later] },
    applied: cache.applied,
    listSequence: at,
  };
}

/** An agent as the answer to request `at` (a rename or an access save) gives it. */
export function withAgent(cache: AgentCache, agent: PersonalAgent, at: number): AgentCache {
  const applied = appliedTo(cache, agent.id);
  const held = cache.list.agents.find((a) => a.id === agent.id);

  if (held === undefined || at <= applied.agent) return cache;

  const next = at > applied.tokens ? agent : { ...agent, credentials: held.credentials };

  return marked(replaced(cache, next), agent.id, {
    agent: at,
    tokens: Math.max(applied.tokens, at),
  });
}

/** A new agent, connected by the request answered as `at`. */
export function withCreated(cache: AgentCache, agent: PersonalAgent, at: number): AgentCache {
  if (cache.list.agents.some((a) => a.id === agent.id)) return cache;

  return marked(
    { ...cache, list: { ...cache.list, agents: [...cache.list.agents, agent] } },
    agent.id,
    { agent: at, tokens: at },
  );
}

/** Agent `agentId` gone, by request `at`; an older load won't bring it back. */
export function withRemoved(cache: AgentCache, agentId: number, at: number): AgentCache {
  return marked(
    {
      ...cache,
      list: { ...cache.list, agents: cache.list.agents.filter((a) => a.id !== agentId) },
    },
    agentId,
    { agent: Math.max(appliedTo(cache, agentId).agent, at) },
  );
}

/**
 * Agent `agentId`'s tokens changed by request `at` (issued, rotated or revoked): merged into the
 * tokens it holds, since token changes don't undo one another.
 */
export function withTokens(
  cache: AgentCache,
  agentId: number,
  change: (credentials: readonly PersonalAgentCredential[]) => PersonalAgentCredential[],
  at: number,
): AgentCache {
  const held = cache.list.agents.find((a) => a.id === agentId);

  if (held === undefined) return cache;

  return marked(replaced(cache, { ...held, credentials: change(held.credentials) }), agentId, {
    tokens: Math.max(appliedTo(cache, agentId).tokens, at),
  });
}
