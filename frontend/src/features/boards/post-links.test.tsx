import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ThreadDetail } from "../../gen/ThreadDetail.ts";
import type { WorkLink } from "../../gen/WorkLink.ts";
import { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { boardDetail } from "../../test/board-fixtures.ts";
import { PostLinks } from "./post-links.tsx";

const pr: WorkLink = {
  id: 11,
  kind: "pull_request",
  label: "acme/api#12",
  url: "https://github.com/acme/api/pull/12",
  pullRequestState: "open",
  title: "Cursor pagination",
  eventStartsAt: null,
  eventTimeZone: null,
  eventCancelled: false,
};

const drive: WorkLink = {
  id: 12,
  kind: "drive_file",
  label: "Launch plan",
  url: "https://docs.google.com/document/d/abcdefghijk",
  pullRequestState: null,
  title: null,
  eventStartsAt: null,
  eventTimeZone: null,
  eventCancelled: false,
};

function deferred<T>() {
  let resolve: (value: T) => void = () => undefined;

  const promise = new Promise<T>((done) => {
    resolve = done;
  });

  return { promise, resolve };
}

async function mount(
  path: string,
  { links = [pr, drive], editable = true }: { links?: WorkLink[]; editable?: boolean } = {},
) {
  const root = createRootRoute({ component: Outlet });
  const pane = () => <PostLinks threadId={7} roomId={4} links={links} editable={editable} />;

  const thread = createRoute({
    getParentRoute: () => root,
    path: "/r/$roomId/t/$threadId",
    component: pane,
  });

  const linking = createRoute({
    getParentRoute: () => root,
    path: "/r/$roomId/t/$threadId/links",
    component: pane,
  });

  const router = createRouter({
    routeTree: root.addChildren([thread, linking]),
    basepath: "/app",
    history: createMemoryHistory({ initialEntries: [path] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());

  return router;
}

// SAFETY: the "button" role is only ever a <button> here.
const button = (name: string) => screen.getByRole("button", { name }) as HTMLButtonElement;

afterEach(() => vi.restoreAllMocks());

describe("a post's links", () => {
  it("removes one link at a time, with the other controls disabled until it answers", async () => {
    const removal = deferred<ThreadDetail>();
    const remove = vi.spyOn(actions.work, "removeLink").mockReturnValue(removal.promise);
    const user = userEvent.setup();

    await mount("/app/r/4/t/7");
    await user.click(button("Remove link acme/api#12"));

    expect(remove).toHaveBeenCalledWith(7, 11);
    expect(button("Remove link Launch plan").disabled).toBe(true);
    expect(button("Link").disabled).toBe(true);

    await user.click(button("Remove link Launch plan"));
    expect(remove).toHaveBeenCalledTimes(1);

    await act(async () => removal.resolve(boardDetail()));
    expect(button("Remove link Launch plan").disabled).toBe(false);
  });

  it("opens the form at the links URL and links a pull request, then closes it", async () => {
    vi.spyOn(actions.work, "linkForm").mockResolvedValue({ events: [] });

    const add = vi.spyOn(actions.work, "addLink").mockResolvedValue(boardDetail());
    const user = userEvent.setup();
    const router = await mount("/app/r/4/t/7/links");

    await user.type(
      screen.getByLabelText("Pull request URL"),
      "https://github.com/acme/api/pull/13",
    );
    await user.click(button("Link pull request"));

    expect(add).toHaveBeenCalledWith(7, {
      kind: "pull_request",
      pullRequestUrl: "https://github.com/acme/api/pull/13",
    });
    expect(router.history.location.pathname).toBe("/app/r/4/t/7");
    expect(screen.queryByRole("form", { name: "Link to this work" })).toBeNull();
  });

  it("shows the server's message under the field it names and keeps the form open", async () => {
    vi.spyOn(actions.work, "linkForm").mockResolvedValue({ events: [] });

    const message = "Enter a GitHub pull request URL, like https://github.com/owner/repo/pull/123.";

    vi.spyOn(actions.work, "addLink").mockRejectedValue(
      new ActionError("Validation", message, { pullRequestUrl: [message] }),
    );

    const user = userEvent.setup();
    const router = await mount("/app/r/4/t/7/links");

    await user.type(screen.getByLabelText("Pull request URL"), "https://example.com/x");
    await user.click(button("Link pull request"));

    expect(await screen.findByText(message)).toBeDefined();
    expect(screen.getByLabelText("Pull request URL").getAttribute("aria-invalid")).toBe("true");
    expect(router.history.location.pathname).toBe("/app/r/4/t/7/links");
  });

  it("links one of the room's upcoming events, the first chosen to start with", async () => {
    vi.spyOn(actions.work, "linkForm").mockResolvedValue({
      events: [
        { id: 31, title: "Planning", startsAt: "2026-10-09T15:00:00.000Z", timeZone: "UTC" },
        { id: 32, title: "Retro", startsAt: "2026-10-10T15:00:00.000Z", timeZone: "UTC" },
      ],
    });

    const add = vi.spyOn(actions.work, "addLink").mockResolvedValue(boardDetail());
    const user = userEvent.setup();

    await mount("/app/r/4/t/7/links");
    await user.click(screen.getByRole("tab", { name: "Event" }));
    await user.selectOptions(await screen.findByRole("combobox", { name: "Event" }), "32");
    await user.click(button("Link event"));

    expect(add).toHaveBeenCalledWith(7, { kind: "event", eventId: 32 });
  });

  it("says when the room has no upcoming events, with nothing to submit", async () => {
    vi.spyOn(actions.work, "linkForm").mockResolvedValue({ events: [] });

    const user = userEvent.setup();

    await mount("/app/r/4/t/7/links");
    await user.click(screen.getByRole("tab", { name: "Event" }));

    expect(await screen.findByText("No upcoming events in this room.")).toBeDefined();
    expect(button("Link event").disabled).toBe(true);
  });

  it("links a Drive file", async () => {
    vi.spyOn(actions.work, "linkForm").mockResolvedValue({ events: [] });

    const add = vi.spyOn(actions.work, "addLink").mockResolvedValue(boardDetail());
    const user = userEvent.setup();

    await mount("/app/r/4/t/7/links");
    await user.click(screen.getByRole("tab", { name: "Drive file" }));
    await user.type(
      screen.getByLabelText("Drive file URL"),
      "https://docs.google.com/document/d/abcdefghijk",
    );
    await user.click(button("Link Drive file"));

    expect(add).toHaveBeenCalledWith(7, {
      kind: "drive_file",
      driveUrl: "https://docs.google.com/document/d/abcdefghijk",
    });
  });

  it("is read-only without the right to change it", async () => {
    await mount("/app/r/4/t/7/links", { editable: false });

    expect(screen.getByText("acme/api#12")).toBeDefined();
    expect(screen.queryByRole("button", { name: /Remove link/ })).toBeNull();
    expect(screen.queryByRole("button", { name: "Link" })).toBeNull();
    expect(screen.queryByRole("form", { name: "Link to this work" })).toBeNull();
  });
});
