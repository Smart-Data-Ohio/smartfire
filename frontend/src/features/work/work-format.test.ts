import { describe, expect, it } from "vitest";
import { historyFixture, linkFixture, tolerated } from "./test-fixtures.ts";
import {
  eventTimeLabel,
  historyText,
  linkAccessibleName,
  linkDetail,
  linkIcon,
  workStatusLabel,
} from "./work-format.ts";
import { parseWorkSearch } from "./work-search.ts";

describe("work wording", () => {
  it("labels the four statuses, and one this build doesn't know", () => {
    expect(workStatusLabel("in_progress")).toBe("In progress");
    expect(workStatusLabel("unknown")).toBe("Unknown status");
  });

  it("writes history lines as the classic page does", () => {
    expect(historyText(historyFixture(1))).toBe(
      "status Planned → In progress · owner Unassigned → User 2",
    );
    expect(
      historyText(
        historyFixture(2, {
          fromStatus: "in_progress",
          toStatus: "in_progress",
          fromOwner: { userId: 2, name: "User 2" },
          toOwner: { userId: 2, name: "User 2" },
          note: "Waiting on legal",
        }),
      ),
    ).toBe("Note: Waiting on legal");
    expect(
      historyText(
        historyFixture(3, {
          kind: "handoff",
          fromOwner: { userId: 2, name: "User 2" },
          toOwner: { userId: 9, name: "Ember" },
          handoff: { summary: "Over to you", linkCount: 1, questionCount: 2 },
        }),
      ),
    ).toBe("handed off owner User 2 → Ember · Over to you (1 link, 2 open questions)");
    expect(historyText(historyFixture(4, { kind: "result" }))).toBe("updated the result");
    expect(
      historyText(historyFixture(5, { fromStatus: null, toStatus: tolerated(), toOwner: null })),
    ).toBe("status Ordinary thread → Unknown status");
    // A snapshot with no name recorded reads "Unassigned".
    expect(
      historyText(
        historyFixture(6, {
          fromStatus: "done",
          toStatus: "done",
          fromOwner: { userId: 4, name: null },
          toOwner: { userId: 2, name: "User 2" },
        }),
      ),
    ).toBe("owner Unassigned → User 2");
    expect(historyText(historyFixture(7, { kind: tolerated(), note: null }))).toBe(
      "changed the work",
    );
  });

  it("describes links by kind, tolerating kinds and states it doesn't know", () => {
    expect(linkIcon("pull_request")).toBe("git-pull-request");
    expect(linkIcon("unknown")).toBe("link");
    expect(linkDetail(linkFixture(1, { pullRequestState: "merged" }))).toBe("Merged");
    expect(linkDetail(linkFixture(1, { pullRequestState: tolerated() }))).toBeNull();

    const event = linkFixture(2, {
      kind: "event",
      label: "Launch review",
      url: "/rooms/4/events/2",
      pullRequestState: null,
      title: null,
      eventStartsAt: "2026-10-07T19:00:00.000Z",
      eventTimeZone: "America/New_York",
    });

    expect(linkDetail(event)).toBe(eventTimeLabel("2026-10-07T19:00:00.000Z", "America/New_York"));
    expect(linkDetail({ ...event, eventCancelled: true })).toBe("Cancelled");
    expect(linkAccessibleName({ ...event, eventCancelled: true })).toBe(
      "Event, Launch review, Cancelled",
    );
  });

  it("falls back to the viewer's zone for a zone the browser doesn't know", () => {
    expect(eventTimeLabel("2026-10-07T19:00:00.000Z", "Not/AZone")).toBe(
      eventTimeLabel("2026-10-07T19:00:00.000Z", null),
    );
  });

  it("reads the work page's tab from the URL, defaulting to open", () => {
    expect(parseWorkSearch({ state: "agents" })).toEqual({ state: "agents" });
    expect(parseWorkSearch({ state: "open" })).toEqual({ state: undefined });
    expect(parseWorkSearch({ state: "nonsense" })).toEqual({ state: undefined });
    expect(parseWorkSearch({})).toEqual({ state: undefined });
  });
});
