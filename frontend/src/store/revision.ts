/** Records use fixed-width server timestamps; arrival and request order do not matter. */
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

/** Scalar field equality for preserving an unchanged optimistic display copy. */
export function sameRecord<T extends object>(left: T, right: T): boolean {
  const entries = Object.entries(left);
  const other = new Map(Object.entries(right));

  return entries.length === other.size && entries.every(([key, value]) => other.get(key) === value);
}
