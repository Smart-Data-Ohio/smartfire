/**
 * Settings → Agents' copy of what the server holds.
 *
 * An agent's profile and access: latest sent wins. Every rename, access save, removal and reload
 * of the whole list takes a number from `nextSequence()` when it is sent, and an answer lands only
 * where no answer to a later-sent request has landed. So a slow reload never puts back an older
 * name, and an older reload can't bring back a removed agent (it keeps its number).
 *
 * Its tokens: they only ever move forward, from active to expired to revoked, and are never
 * deleted. So every answer's tokens are merged in by that, not by when it was sent (the server may
 * carry out two requests in the other order): a token any answer showed revoked stays revoked, one
 * an answer doesn't mention stays as it is, and a new one is added.
 *
 * And which agents a save went unanswered for: it may or may not have landed, so the cache can't
 * tell what the server holds for them until an answer to a request sent after it says.
 */
import type { PersonalAgent } from "../../gen/PersonalAgent.ts";
import type { PersonalAgentCredential } from "../../gen/PersonalAgentCredential.ts";
import type { PersonalAgentList } from "../../gen/PersonalAgentList.ts";

export interface AgentCache {
  readonly list: PersonalAgentList;
  /** The newest request applied to each agent's profile and access, removed agents included. */
  readonly applied: ReadonlyMap<number, number>;
  /** The newest whole-list load applied: it speaks for everything not written to since. */
  readonly listSequence: number;
  /** Agents with a save that went unanswered, by the number that save was sent as. */
  readonly unconfirmed: ReadonlyMap<number, number>;
}

let sequence = 0;

/** The next request's number; numbers only grow. */
export function nextSequence(): number {
  sequence += 1;

  return sequence;
}

/** The newest request applied to agent `agentId`'s profile and access. */
function profileAt(cache: AgentCache, agentId: number): number {
  return Math.max(cache.applied.get(agentId) ?? 0, cache.listSequence);
}

/** How far along a token is; it never goes back. */
const STAGE: Readonly<Record<PersonalAgentCredential["state"], number>> = {
  active: 0,
  expired: 1,
  revoked: 2,
};

/** `held` tokens with `incoming` ones merged in: each the furthest along, new ones added. */
function mergedTokens(
  held: readonly PersonalAgentCredential[],
  incoming: readonly PersonalAgentCredential[],
): PersonalAgentCredential[] {
  const merged = held.map((token) => {
    const fresh = incoming.find((t) => t.id === token.id);

    return fresh === undefined || STAGE[fresh.state] < STAGE[token.state] ? token : fresh;
  });

  const added = incoming.filter((fresh) => !held.some((token) => token.id === fresh.id));

  return [...merged, ...added];
}

/** `unconfirmed` without agent `agentId`, if request `at` was sent after its unanswered save. */
function confirmed(
  unconfirmed: ReadonlyMap<number, number>,
  agentId: number,
  at: number,
): ReadonlyMap<number, number> {
  const since = unconfirmed.get(agentId);

  if (since === undefined || at <= since) return unconfirmed;

  const next = new Map(unconfirmed);

  next.delete(agentId);

  return next;
}

/** A whole-list load sent as request `at`, over what `cache` holds (if anything yet). */
export function withList(
  cache: AgentCache | null,
  fresh: PersonalAgentList,
  at: number,
): AgentCache {
  if (cache === null) {
    return { list: fresh, applied: new Map(), listSequence: at, unconfirmed: new Map() };
  }

  if (at <= cache.listSequence) return cache;

  const listed = new Set(fresh.agents.map((agent) => agent.id));

  const agents = fresh.agents.flatMap((agent) => {
    const held = cache.list.agents.find((a) => a.id === agent.id);
    const profileNewer = (cache.applied.get(agent.id) ?? 0) > at;

    // Removed after this load was sent.
    if (held === undefined) return profileNewer ? [] : [agent];

    const credentials = mergedTokens(held.credentials, agent.credentials);

    return [{ ...(profileNewer ? held : agent), credentials }];
  });

  // Connected after this load was sent, so it couldn't list them.
  const later = cache.list.agents.filter(
    (agent) => !listed.has(agent.id) && (cache.applied.get(agent.id) ?? 0) > at,
  );

  const unconfirmed = [...cache.unconfirmed.keys()].reduce(
    (current, agentId) => confirmed(current, agentId, at),
    cache.unconfirmed,
  );

  return {
    list: { ...fresh, agents: [...agents, ...later] },
    applied: cache.applied,
    listSequence: at,
    unconfirmed,
  };
}

/** An agent as the answer to request `at` (a rename or an access save) gives it. */
export function withAgent(cache: AgentCache, agent: PersonalAgent, at: number): AgentCache {
  const held = cache.list.agents.find((a) => a.id === agent.id);

  if (held === undefined) return cache;

  const credentials = mergedTokens(held.credentials, agent.credentials);
  const newer = at > profileAt(cache, agent.id);
  const next = { ...(newer ? agent : held), credentials };
  const applied = new Map(cache.applied);

  if (newer) applied.set(agent.id, at);

  return {
    ...cache,
    list: { ...cache.list, agents: cache.list.agents.map((a) => (a.id === agent.id ? next : a)) },
    applied,
    unconfirmed: confirmed(cache.unconfirmed, agent.id, at),
  };
}

/** A save for agent `agentId`, sent as request `at`, that went unanswered. */
export function withUnconfirmed(cache: AgentCache, agentId: number, at: number): AgentCache {
  if (!cache.list.agents.some((a) => a.id === agentId)) return cache;

  const unconfirmed = new Map(cache.unconfirmed);

  unconfirmed.set(agentId, Math.max(unconfirmed.get(agentId) ?? 0, at));

  return { ...cache, unconfirmed };
}

/** Whether a save for agent `agentId` went unanswered and nothing sent since has answered. */
export function isUnconfirmed(cache: AgentCache, agentId: number): boolean {
  return cache.unconfirmed.has(agentId);
}

/** A new agent, connected by the request answered as `at`. */
export function withCreated(cache: AgentCache, agent: PersonalAgent, at: number): AgentCache {
  if (cache.list.agents.some((a) => a.id === agent.id)) return cache;

  const applied = new Map(cache.applied);

  applied.set(agent.id, at);

  return { ...cache, list: { ...cache.list, agents: [...cache.list.agents, agent] }, applied };
}

/** Agent `agentId` gone, by request `at`; an older load won't bring it back. */
export function withRemoved(cache: AgentCache, agentId: number, at: number): AgentCache {
  const applied = new Map(cache.applied);
  const unconfirmed = new Map(cache.unconfirmed);

  applied.set(agentId, Math.max(profileAt(cache, agentId), at));
  unconfirmed.delete(agentId);

  return {
    ...cache,
    list: { ...cache.list, agents: cache.list.agents.filter((a) => a.id !== agentId) },
    applied,
    unconfirmed,
  };
}

/**
 * Agent `agentId`'s tokens as an answer gives them: an issued token, a rotated one with the one it
 * replaced, or a revocation's whole list.
 */
export function withTokens(
  cache: AgentCache,
  agentId: number,
  incoming: readonly PersonalAgentCredential[],
): AgentCache {
  return {
    ...cache,
    list: {
      ...cache.list,
      agents: cache.list.agents.map((a) =>
        a.id === agentId ? { ...a, credentials: mergedTokens(a.credentials, incoming) } : a,
      ),
    },
  };
}
