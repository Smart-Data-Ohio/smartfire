import type { Settings } from "../gen/Settings.ts";

type SnapshotVersion = Pick<Settings, "revision" | "evaluatedAt">;

/** Evaluation times have fixed precision in UTC, so string order is time order. */
export function compareSnapshots(incoming: SnapshotVersion, held: SnapshotVersion): number {
  return (
    incoming.revision - held.revision ||
    (incoming.evaluatedAt < held.evaluatedAt ? -1 : incoming.evaluatedAt > held.evaluatedAt ? 1 : 0)
  );
}
