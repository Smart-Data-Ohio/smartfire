import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { GithubSubscriptionList } from "../../gen/GithubSubscriptionList.ts";
import type { InboundEmail } from "../../gen/InboundEmail.ts";
import { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { GithubSubscriptions } from "./github-subscriptions.tsx";
import { InboundEmailSection } from "./inbound-email.tsx";
import { resetIntegrationSessions } from "./integration-session.ts";

const EVENTS: GithubSubscriptionList["events"] = [
  { key: "opened", label: "Opened", selectedByDefault: true },
  { key: "merged", label: "Merged", selectedByDefault: true },
  { key: "closed", label: "Closed", selectedByDefault: false },
  { key: "review_requested", label: "Review requested", selectedByDefault: true },
  { key: "review_submitted", label: "Review submitted", selectedByDefault: false },
  { key: "checks_failed", label: "Checks failed", selectedByDefault: true },
];

function list(change?: Partial<GithubSubscriptionList>): GithubSubscriptionList {
  return {
    subscriptions: [],
    administrator: true,
    connectPath: "/github/app/connect",
    events: EVENTS,
    ...change,
  };
}

afterEach(() => {
  resetIntegrationSessions();
  vi.restoreAllMocks();
});

// SAFETY: these labels name <input type="checkbox"> elements, whose `checked` the tests read.
const checkbox = (name: string | RegExp) =>
  screen.getByRole("checkbox", { name }) as HTMLInputElement;

describe("GitHub subscriptions", () => {
  it("subscribes with the classic defaults and unsubscribes after a confirm", async () => {
    vi.spyOn(actions.rooms, "githubSubscriptions").mockResolvedValue(list());

    const subscribe = vi.spyOn(actions.rooms, "subscribeRepository").mockResolvedValue({
      id: 4,
      fullName: "rails/rails",
      events: ["opened", "merged", "review_requested", "checks_failed"],
    });

    const unsubscribe = vi.spyOn(actions.rooms, "unsubscribeRepository").mockResolvedValue({
      id: 4,
      fullName: "rails/rails",
      events: ["opened"],
    });

    const user = userEvent.setup();

    render(<GithubSubscriptions roomId={7} />);
    expect(await screen.findByText("No repositories subscribed yet.")).toBeTruthy();
    expect(checkbox("Opened").checked).toBe(true);
    expect(checkbox("Closed").checked).toBe(false);
    expect(checkbox(/Subscribe without verifying/)).toBeTruthy();

    await user.type(screen.getByLabelText("Repository"), "Rails/Rails");
    await user.click(screen.getByRole("button", { name: "Subscribe" }));

    expect(subscribe).toHaveBeenCalledWith(7, {
      fullName: "Rails/Rails",
      events: ["opened", "merged", "review_requested", "checks_failed"],
      skipAccessCheck: false,
    });
    expect(await screen.findByText("rails/rails")).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "Remove rails/rails" }));
    await user.click(
      within(screen.getByRole("alertdialog", { name: "Unsubscribe rails/rails?" })).getByRole(
        "button",
        { name: "Remove" },
      ),
    );
    expect(unsubscribe).toHaveBeenCalledWith(7, 4);
    expect(await screen.findByText("No repositories subscribed yet.")).toBeTruthy();
  });

  it("keeps Enter in the repository field from leaving the section, and links GitHub when unlinked", async () => {
    vi.spyOn(actions.rooms, "githubSubscriptions").mockResolvedValue(list());

    const subscribe = vi
      .spyOn(actions.rooms, "subscribeRepository")
      .mockRejectedValue(
        new ActionError(
          "Validation",
          "Could not subscribe: link your GitHub account on your profile so it can confirm you can read rails/rails.",
          { github: ["not_linked"] },
        ),
      );

    const user = userEvent.setup();

    render(<GithubSubscriptions roomId={7} />);
    const field = await screen.findByLabelText("Repository");

    await user.type(field, "rails/rails");
    field.focus();
    await user.keyboard("{Enter}");

    expect(subscribe).toHaveBeenCalledOnce();
    expect((await screen.findByRole("alert")).textContent).toContain("link your GitHub account");
    expect(
      screen.getByRole("link", { name: "Link your GitHub account" }).getAttribute("href"),
    ).toBe("/github/app/connect");
  });

  it("refuses someone who cannot manage the room", async () => {
    vi.spyOn(actions.rooms, "githubSubscriptions").mockRejectedValue(
      new ActionError("Forbidden", "Not allowed"),
    );

    render(<GithubSubscriptions roomId={7} />);
    expect(await screen.findByText(/administrators can manage GitHub/)).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Subscribe" })).toBeNull();
  });

  it("shows an error with a retry, and hides the administrator override otherwise", async () => {
    const load = vi
      .spyOn(actions.rooms, "githubSubscriptions")
      .mockRejectedValueOnce(new ActionError("NetworkError", "offline"))
      .mockResolvedValue(list({ administrator: false }));

    const user = userEvent.setup();

    render(<GithubSubscriptions roomId={7} />);
    expect((await screen.findByRole("alert")).textContent).toContain("offline");
    await user.click(screen.getByRole("button", { name: "Try again" }));
    expect(await screen.findByText("No repositories subscribed yet.")).toBeTruthy();
    expect(screen.queryByRole("checkbox", { name: /Subscribe without verifying/ })).toBeNull();
    expect(load).toHaveBeenCalledTimes(2);
  });
});

describe("inbound email", () => {
  it("shows the address and rotates it after a confirm", async () => {
    const load = vi.spyOn(actions.rooms, "inboundEmail").mockResolvedValue({
      enabled: true,
      address: "room-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa@mail.campfire.test",
    } satisfies InboundEmail);

    const rotate = vi.spyOn(actions.rooms, "rotateInboundEmail").mockResolvedValue({
      enabled: true,
      address: "room-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb@mail.campfire.test",
    });

    const user = userEvent.setup();

    render(<InboundEmailSection roomId={7} />);
    expect(await screen.findByText(/room-aaaa/)).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Rotate address" }));
    await user.click(
      within(screen.getByRole("alertdialog")).getByRole("button", { name: "Rotate address" }),
    );
    expect(rotate).toHaveBeenCalledWith(7);
    expect(await screen.findByText(/room-bbbb/)).toBeTruthy();
    expect(load).toHaveBeenCalledOnce();
  });

  it("creates an address when there is none, and explains a workspace with no domain", async () => {
    vi.spyOn(actions.rooms, "inboundEmail").mockResolvedValue({ enabled: true, address: null });

    const rotate = vi.spyOn(actions.rooms, "rotateInboundEmail").mockResolvedValue({
      enabled: true,
      address: "room-cccccccccccccccccccccccccccccccc@mail.campfire.test",
    });

    const user = userEvent.setup();

    const view = render(<InboundEmailSection roomId={8} />);

    expect(await screen.findByText(/No email address yet/)).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Create email address" }));
    expect(rotate).toHaveBeenCalledWith(8);
    expect(await screen.findByText(/room-cccc/)).toBeTruthy();

    vi.spyOn(actions.rooms, "inboundEmail").mockResolvedValue({ enabled: false, address: null });
    view.rerender(<InboundEmailSection roomId={9} />);
    expect(await screen.findByText(/not configured/)).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Create email address" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Rotate address" })).toBeNull();
  });

  it("refuses someone who cannot manage the address, and retries a failed load", async () => {
    const load = vi
      .spyOn(actions.rooms, "inboundEmail")
      .mockRejectedValueOnce(new ActionError("Forbidden", "Not allowed"))
      .mockRejectedValueOnce(new ActionError("NetworkError", "offline"))
      .mockResolvedValue({ enabled: true, address: null });

    const user = userEvent.setup();

    const view = render(<InboundEmailSection roomId={7} />);

    expect(await screen.findByText(/administrators can manage its email/)).toBeTruthy();

    view.rerender(<InboundEmailSection roomId={8} />);
    expect((await screen.findByRole("alert")).textContent).toContain("offline");
    await user.click(screen.getByRole("button", { name: "Try again" }));
    expect(await screen.findByText(/No email address yet/)).toBeTruthy();
    expect(load).toHaveBeenCalledTimes(3);
  });
});
