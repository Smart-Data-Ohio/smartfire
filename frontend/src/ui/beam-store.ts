/**
 * Only one Beam animates at a time: a beam on every agent card at once would be noise. Claims
 * stack, and the most recent claimer owns the beam until it lets go.
 */
let claims: readonly string[] = [];

const listeners = new Set<() => void>();

function emit(next: readonly string[]): void {
  claims = next;

  for (const listener of listeners) {
    listener();
  }
}

export function claimBeam(id: string): void {
  emit([...claims.filter((claim) => claim !== id), id]);
}

export function releaseBeam(id: string): void {
  if (claims.includes(id)) {
    emit(claims.filter((claim) => claim !== id));
  }
}

export function subscribeBeam(listener: () => void): () => void {
  listeners.add(listener);

  return () => {
    listeners.delete(listener);
  };
}

export function beamOwner(): string | null {
  return claims.at(-1) ?? null;
}
