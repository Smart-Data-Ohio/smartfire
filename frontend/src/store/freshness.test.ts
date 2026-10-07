import { describe, expect, it } from "vitest";
import {
  emptyFreshness,
  finishRead,
  land,
  markLocal,
  membership,
  modifiedAt,
  placeId,
  receive,
  replay,
  startRead,
} from "./freshness.ts";

const key = "work:7";

const same = (left: string, right: string) => left === right;

const order = (left: number, right: number) => right - left;

function copies(held: string, incoming: string) {
  return { held, incoming, same };
}

describe("shared freshness", () => {
  it("rejects a read after a local write and accepts the write's equal echo and response", () => {
    const read = startRead(emptyFreshness);
    const local = markLocal(read.freshness, key);
    const write = startRead(local);
    const echo = receive(write.freshness, key, copies("done", "done"));
    const response = land(echo.freshness, key, write.ticket, copies("done", "done"));
    const stale = land(response.freshness, key, read.ticket, copies("done", "blocked"));

    expect(modifiedAt(echo.freshness, key)).toBe(modifiedAt(local, key));
    expect(response.value).toBe("done");
    expect(finishRead(response.freshness, write.ticket).rejected).toBe(false);
    expect(stale.value).toBe("done");
    expect(finishRead(stale.freshness, read.ticket).rejected).toBe(true);
  });

  it("orders concurrent responses and marks a rollback even when its facts match", () => {
    const first = startRead(emptyFreshness);
    const second = startRead(first.freshness);
    const newer = land(second.freshness, key, second.ticket, copies("planned", "done"));
    const older = land(newer.freshness, key, first.ticket, copies("done", "blocked"));
    const rollback = markLocal(older.freshness, key);

    expect(older.value).toBe("done");
    expect(modifiedAt(rollback, key)).toBeGreaterThan(second.ticket);
  });

  it("lets timestamps win in both directions, with tickets deciding ties and missing timestamps", () => {
    const read = startRead(emptyFreshness);
    const timestamp = (value: string) => value;
    const event = receive(read.freshness, key, { ...copies("T00", "T10"), timestamp });

    const response = land(event.freshness, key, read.ticket, {
      ...copies("T10", "T20"),
      timestamp,
    });

    const delayed = receive(response.freshness, key, { ...copies("T20", "T10"), timestamp });
    const tied = land(event.freshness, key, read.ticket, { ...copies("T10", "T10"), timestamp });

    expect(response.value).toBe("T20");
    expect(delayed.value).toBe("T20");
    expect(finishRead(response.freshness, read.ticket).rejected).toBe(false);
    expect(finishRead(tied.freshness, read.ticket).rejected).toBe(true);
    expect(land(event.freshness, key, read.ticket, copies("T10", "absent")).value).toBe("T10");
  });

  it("reports a partial rejection while allowing unrelated keys to merge", () => {
    const read = startRead(emptyFreshness);
    const changed = markLocal(read.freshness, key);
    const held = land(changed, key, read.ticket, copies("done", "blocked"));
    const other = land(held.freshness, "work:8", read.ticket, copies("planned", "done"));

    expect(held.value).toBe("done");
    expect(other.value).toBe("done");
    expect(finishRead(other.freshness, read.ticket).rejected).toBe(true);
  });

  it("replays additions and removals and prunes only deltas no outstanding read needs", () => {
    const first = startRead(emptyFreshness, "approved");
    const changed = markLocal(first.freshness, key);
    const added = membership(changed, "approved", [2], [3, 2]);
    const second = startRead(added, "approved");
    const removed = membership(markLocal(second.freshness, key), "approved", [3, 2], [3]);
    const page = replay(removed, "approved", first.ticket, [2], order);
    const finished = finishRead(page.freshness, first.ticket);

    expect(page.ids).toEqual([3]);
    expect(finished.rejected).toBe(true);
    expect(finished.freshness.deltas).toHaveLength(1);
    expect(replay(finished.freshness, "approved", second.ticket, [3, 2], order).ids).toEqual([3]);
    expect(finishRead(finished.freshness, second.ticket).freshness.deltas).toEqual([]);
  });

  it("timestamps membership changes even when the entity echo is equal", () => {
    const read = startRead(emptyFreshness, "approved");
    const echo = receive(read.freshness, key, copies("done", "done"));
    const changed = membership(echo.freshness, "approved", [], [3]);
    const page = replay(changed, "approved", read.ticket, [], order);

    expect(page.ids).toEqual([3]);
    expect(finishRead(page.freshness, read.ticket).rejected).toBe(true);
  });

  it("prunes a list's deltas while an unrelated read is still outstanding", () => {
    const unrelated = startRead(emptyFreshness);
    const read = startRead(unrelated.freshness, "approved");
    const changed = membership(read.freshness, "approved", [], [3]);

    expect(finishRead(changed, read.ticket).freshness.deltas).toEqual([]);
  });

  it("keeps an existing member's place", () => {
    const ids = [100, 52, 51];

    expect(placeId(ids, 51, true, order)).toBe(ids);
  });
});
