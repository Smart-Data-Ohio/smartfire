/** The one comparison rule for server-revisioned users, work facts, approvals and agent status. */
export interface Revisioned {
  readonly updatedAt: string;
}

/** Equal revisions land too, so another copy can fill missing fields. */
export function landsOver<T extends Revisioned>(
  stored: T | null | undefined,
  incoming: T,
): boolean {
  return stored == null || incoming.updatedAt >= stored.updatedAt;
}

export function mergeRevision<T extends Revisioned>(stored: T | null | undefined, incoming: T): T {
  return landsOver(stored, incoming) ? incoming : (stored ?? incoming);
}

/** Scalar field equality preserves unchanged optimistic copies; property order does not matter. */
export function sameRecord<T extends object>(left: T, right: T): boolean {
  const entries = Object.entries(left);
  const other = new Map(Object.entries(right));

  return entries.length === other.size && entries.every(([key, value]) => other.get(key) === value);
}
