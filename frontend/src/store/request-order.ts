let nextSequence = 0;

/** Take a ticket before starting a snapshot request, including writes that return snapshots. */
export function beginSnapshotRequest(): number {
  return ++nextSequence;
}
