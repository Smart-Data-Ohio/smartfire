import { describe, expect, it } from "vitest";
import type { SlackIssue } from "../../gen/SlackIssue.ts";
import type { SlackRun } from "../../gen/SlackRun.ts";
import type { SlackRunPage } from "../../gen/SlackRunPage.ts";
import { MAX_ISSUE_PAGES, readPages, serialQueue } from "./slack-pages.ts";

const RUN: SlackRun = {
  id: 12,
  kind: "workspace",
  mode: "dry_run",
  status: "completed",
  title: "Workspace dry run #12",
  startedBy: "Grace",
  createdAt: "2026-10-06T12:00:00.000Z",
  startedAt: "2026-10-06T12:00:05.000Z",
  finishedAt: "2026-10-06T12:01:00.000Z",
  phase: null,
  current: null,
  queuedBehind: false,
  people: null,
  counts: null,
  apiCalls: 41,
  issuesCount: 0,
  error: null,
  active: false,
  cancellable: false,
  undoable: false,
  undoBlockedReason: null,
  planReady: true,
  catchUp: false,
  conversations: [],
};

const issue = (n: number): SlackIssue => ({
  level: "warning",
  slackRef: `F${n}`,
  message: `File ${n} wasn't imported.`,
});

/** A run of `total` pages, two issues each, recording every page asked for. */
function pager(total: number, empty: ReadonlySet<number> = new Set()) {
  const asked: (number | null)[] = [];

  const fetchPage = async (page: number | null): Promise<SlackRunPage> => {
    asked.push(page);

    const n = page ?? 1;
    const issues = empty.has(n) ? [] : [issue(n * 2 - 1), issue(n * 2)];

    return { run: RUN, issues, nextPage: n < total ? n + 1 : null };
  };

  return { asked, fetchPage };
}

describe("reading a run's pages", () => {
  it("reads through the page asked for, and no further than the last", async () => {
    const short = pager(5);
    const read = await readPages(short.fetchPage, 3, () => true);

    expect(short.asked).toEqual([null, 2, 3]);
    expect(read?.issues.map((row) => row.slackRef)).toEqual(["F1", "F2", "F3", "F4", "F5", "F6"]);
    expect(read).toMatchObject({ pages: 3, page: { nextPage: 4 } });

    const all = pager(2);

    expect(await readPages(all.fetchPage, 9, () => true)).toMatchObject({
      pages: 2,
      page: { nextPage: null },
    });
  });

  it("stops at the page limit for a huge page number", async () => {
    const huge = pager(1_000_000);
    const read = await readPages(huge.fetchPage, 1_000_000, () => true);

    expect(huge.asked).toHaveLength(MAX_ISSUE_PAGES);
    expect(read?.pages).toBe(MAX_ISSUE_PAGES);
  });

  it("stops at an empty page", async () => {
    const gap = pager(10, new Set([3]));

    await readPages(gap.fetchPage, 10, () => true);

    expect(gap.asked).toEqual([null, 2, 3]);
  });

  it("asks for nothing more once the page has gone away", async () => {
    const run = pager(10);
    let live = true;

    const fetchPage = async (page: number | null) => {
      const answer = await run.fetchPage(page);

      if (page === 2) live = false;

      return answer;
    };

    expect(await readPages(fetchPage, 10, () => live)).toBeNull();
    expect(run.asked).toEqual([null, 2]);
  });
});

describe("the reads' turns", () => {
  it("starts each read once the one before has settled, failed or not", async () => {
    const serial = serialQueue();
    const order: string[] = [];
    const { promise: held, resolve: release } = Promise.withResolvers<void>();

    const first = serial(async () => {
      order.push("refresh starts");
      await held;
      order.push("refresh lands");

      throw new Error("refused");
    });

    const second = serial(async () => {
      order.push("older starts");

      return "older";
    });

    await Promise.resolve();
    expect(order).toEqual(["refresh starts"]);

    release();

    await expect(first).rejects.toThrow("refused");
    await expect(second).resolves.toBe("older");
    expect(order).toEqual(["refresh starts", "refresh lands", "older starts"]);
  });
});
