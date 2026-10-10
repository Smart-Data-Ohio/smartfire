let nextSequence = 0;

/** Take a ticket before starting a snapshot request, including writes that return snapshots. */
export function beginSnapshotRequest(): number {
  return ++nextSequence;
}

/** A pending or failed newer request does not displace a completed snapshot. */
export function newerSnapshotRequest(started: number, completed: number): boolean {
  return started > completed;
}
