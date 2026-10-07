/**
 * A run's issues a page at a time: reading the pages a classic `?page=N` link asks for, and the
 * order the page's refresh and "Older issues" take turns in.
 */
import type { SlackIssue } from "../../gen/SlackIssue.ts";
import type { SlackRunPage } from "../../gen/SlackRunPage.ts";

/**
 * The most pages read at first. A classic link names the page someone was on; past this the
 * person pages on with "Older issues" rather than the page reading thousands of issues in a row.
 */
export const MAX_ISSUE_PAGES = 20;

/** The pages read so far: the run, its issues in order, where the next page starts. */
export interface ReadPages {
  readonly page: SlackRunPage;
  readonly issues: readonly SlackIssue[];
  readonly pages: number;
}

/**
 * Reads the first page, then the next ones up to `pages` (at most `MAX_ISSUE_PAGES`). It stops
 * early at the last page or an empty one, and answers `null` once `live()` turns false (the page
 * went away, or a newer read took over) without asking for another page.
 */
export async function readPages(
  fetchPage: (page: number | null) => Promise<SlackRunPage>,
  pages: number,
  live: () => boolean,
): Promise<ReadPages | null> {
  const wanted = Math.min(Math.max(pages, 1), MAX_ISSUE_PAGES);
  const first = await fetchPage(null);
  const issues = [...first.issues];
  let last = first;
  let read = 1;

  while (read < wanted && last.nextPage !== null) {
    if (!live()) return null;

    const next = await fetchPage(last.nextPage);

    read += 1;
    last = next;
    issues.push(...next.issues);

    if (next.issues.length === 0) break;
  }

  return live() ? { page: { ...first, nextPage: last.nextPage }, issues, pages: read } : null;
}

/**
 * Runs reads one after another, in the order asked: a refresh that lands while "Older issues" is
 * loading starts once it's in, so it reads that page too, and the other way round.
 */
export function serialQueue(): <T>(work: () => Promise<T>) => Promise<T> {
  let tail: Promise<unknown> = Promise.resolve();

  return <T>(work: () => Promise<T>): Promise<T> => {
    const next = tail.then(work, work);

    tail = next.catch(() => undefined);

    return next;
  };
}
