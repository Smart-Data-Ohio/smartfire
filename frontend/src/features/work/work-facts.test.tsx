import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { userFixture } from "../../api/testing.ts";
import { mutations } from "../../store/store.ts";
import { factsFixture, linkFixture } from "./test-fixtures.ts";
import { WorkLinks, WorkOwner, WorkStatusPill, WorkSummary } from "./work-facts.tsx";

afterEach(() => mutations.reset());

describe("work facts", () => {
  it("gives a status this build doesn't know a neutral pill", () => {
    const { container } = render(<WorkStatusPill status="unknown" />);

    expect(container.querySelector(".work-status")?.getAttribute("data-status")).toBe("unknown");
    expect(screen.getAllByText("Unknown status").length).toBeGreaterThan(0);
  });

  it("names the owner from the facts, and says when they're inactive", () => {
    render(<WorkOwner owner={userFixture(4, "Dana Reyes")} active={false} />);

    expect(screen.getByText("Dana Reyes")).toBeTruthy();
    expect(screen.getByText("owner inactive")).toBeTruthy();
  });

  it("prefers the store's copy of the owner, which follows renames", () => {
    mutations.mergeUsers([userFixture(4, "Dana R.")]);
    render(<WorkOwner owner={userFixture(4, "Dana Reyes")} active />);

    expect(screen.getByText("Dana R.")).toBeTruthy();
    expect(screen.queryByText("owner inactive")).toBeNull();
  });

  it("reads Unassigned with no owner", () => {
    render(<WorkSummary facts={factsFixture({ owner: null, ownerActive: false })} />);

    expect(screen.getByText("Unassigned")).toBeTruthy();
  });

  it("links pull requests and events, but never an unsafe URL", () => {
    render(
      <WorkLinks
        label="Links for Copy"
        runUrl="javascript:alert(1)"
        links={[
          linkFixture(1),
          linkFixture(2, {
            kind: "event",
            label: "Review",
            url: "/rooms/4/events/2",
            pullRequestState: null,
            title: null,
            eventCancelled: true,
          }),
          linkFixture(3, { label: "Sneaky", url: "javascript:alert(1)" }),
        ]}
      />,
    );

    const pull = screen.getByRole("link", { name: /^Pull request, acme\/app#1/ });

    expect(pull.getAttribute("target")).toBe("_blank");
    expect(
      screen.getByRole("link", { name: "Event, Review, Cancelled" }).getAttribute("target"),
    ).toBeNull();
    expect(screen.queryByRole("link", { name: /Sneaky/ })).toBeNull();
    expect(screen.getByText("Sneaky")).toBeTruthy();
    expect(screen.queryByRole("link", { name: /^Run/ })).toBeNull();
  });
});
