import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
  useParams,
} from "@tanstack/react-router";
import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createContext, useContext } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ThreadDetail } from "../../gen/ThreadDetail.ts";
import type { WorkLink } from "../../gen/WorkLink.ts";
import { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { boardDetail } from "../../test/board-fixtures.ts";
import { toastSnapshot } from "../../ui/toast-store.ts";
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
  let reject: (error: Error) => void = () => undefined;

  const promise = new Promise<T>((done, fail) => {
    resolve = done;
    reject = fail;
  });

  return { promise, resolve, reject };
}

const planning = {
  id: 31,
  title: "Planning",
  startsAt: "2026-10-09T15:00:00.000Z",
  timeZone: "UTC",
};

const retro = { id: 32, title: "Retro", startsAt: "2026-10-10T15:00:00.000Z", timeZone: "UTC" };

const Links = createContext<readonly WorkLink[]>([]);

/** The post at the route's thread, keyed by it as the pane's post work keys it. */
function Pane({ editable }: { readonly editable: boolean }) {
  const { threadId } = useParams({ strict: false });
  const links = useContext(Links);

  return (
    <PostLinks
      key={threadId}
      threadId={Number(threadId)}
      roomId={4}
      links={links}
      editable={editable}
    />
  );
}

async function mount(
  path: string,
  { links = [pr, drive], editable = true }: { links?: WorkLink[]; editable?: boolean } = {},
) {
  const root = createRootRoute({ component: Outlet });
  const pane = () => <Pane editable={editable} />;

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

  const view = render(
    <Links.Provider value={links}>
      <RouterProvider router={router} />
    </Links.Provider>,
  );

  await act(() => router.load());

  /** The post's links change (here or in another client). */
  const relink = (next: readonly WorkLink[]) =>
    view.rerender(
      <Links.Provider value={next}>
        <RouterProvider router={router} />
      </Links.Provider>,
    );

  return { router, relink };
}

// SAFETY: the "button" role is only ever a <button> here.
const button = (name: string) => screen.getByRole("button", { name }) as HTMLButtonElement;

// SAFETY: the URL fields are <input>s.
const urlField = (label: string) => screen.getByLabelText(label) as HTMLInputElement;

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
    const { router } = await mount("/app/r/4/t/7/links");

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
    const { router } = await mount("/app/r/4/t/7/links");

    await user.type(screen.getByLabelText("Pull request URL"), "https://example.com/x");
    await user.click(button("Link pull request"));

    expect(await screen.findByText(message)).toBeDefined();
    expect(screen.getByLabelText("Pull request URL").getAttribute("aria-invalid")).toBe("true");
    expect(urlField("Pull request URL").value).toBe("https://example.com/x");
    expect(router.history.location.pathname).toBe("/app/r/4/t/7/links");
  });

  it("keeps the link, and says why, when removing it fails", async () => {
    vi.spyOn(actions.work, "removeLink").mockRejectedValue(
      new ActionError("ServerError", "Something went wrong.", {}),
    );

    const user = userEvent.setup();

    await mount("/app/r/4/t/7");
    await user.click(button("Remove link acme/api#12"));

    await waitFor(() =>
      expect(toastSnapshot().at(-1)).toMatchObject({
        title: "Couldn't remove acme/api#12",
        description: "Something went wrong.",
        tone: "danger",
      }),
    );
    expect(screen.getByText("acme/api#12")).toBeDefined();
    expect(button("Remove link acme/api#12").disabled).toBe(false);
  });

  it("leaves another post's draft and pending link alone when an earlier removal fails late", async () => {
    vi.spyOn(actions.work, "linkForm").mockResolvedValue({ events: [] });

    const removal = deferred<ThreadDetail>();
    const saving = deferred<ThreadDetail>();

    vi.spyOn(actions.work, "removeLink").mockReturnValue(removal.promise);
    vi.spyOn(actions.work, "addLink").mockReturnValue(saving.promise);

    const user = userEvent.setup();
    const { router } = await mount("/app/r/4/t/7");

    await user.click(button("Remove link acme/api#12"));

    // The reader moves to another post and starts linking there while the removal is out.
    await act(() =>
      router.navigate({ to: "/r/$roomId/t/$threadId/links", params: { roomId: 4, threadId: 8 } }),
    );
    await user.type(urlField("Pull request URL"), "https://github.com/acme/web/pull/2");
    await user.click(button("Link pull request"));
    expect(button("Link pull request").disabled).toBe(true);

    await act(async () =>
      removal.reject(new ActionError("ServerError", "Something went wrong.", {})),
    );

    // The toast names the old post's link; this post's draft and pending link are untouched.
    expect(toastSnapshot().at(-1)).toMatchObject({ title: "Couldn't remove acme/api#12" });
    expect(router.history.location.pathname).toBe("/app/r/4/t/8/links");
    expect(urlField("Pull request URL").value).toBe("https://github.com/acme/web/pull/2");
    expect(button("Link pull request").disabled).toBe(true);
    expect(button("Remove link acme/api#12").disabled).toBe(true);
  });

  it("leaves another post's draft alone when an earlier link answers late", async () => {
    vi.spyOn(actions.work, "linkForm").mockResolvedValue({ events: [] });

    const saving = deferred<ThreadDetail>();

    vi.spyOn(actions.work, "addLink").mockReturnValue(saving.promise);

    const user = userEvent.setup();
    const { router } = await mount("/app/r/4/t/7/links");

    await user.type(
      screen.getByLabelText("Pull request URL"),
      "https://github.com/acme/api/pull/13",
    );
    await user.click(button("Link pull request"));

    // The reader moves to another post and starts a link there before the first one answers.
    await act(() =>
      router.navigate({ to: "/r/$roomId/t/$threadId/links", params: { roomId: 4, threadId: 8 } }),
    );
    await user.type(
      screen.getByLabelText("Pull request URL"),
      "https://github.com/acme/web/pull/2",
    );
    await act(async () => saving.resolve(boardDetail()));

    expect(router.history.location.pathname).toBe("/app/r/4/t/8/links");
    expect(urlField("Pull request URL").value).toBe("https://github.com/acme/web/pull/2");
  });

  it("offers the events again when the links change, keeping the draft", async () => {
    const form = vi
      .spyOn(actions.work, "linkForm")
      .mockResolvedValueOnce({ events: [planning] })
      .mockResolvedValueOnce({ events: [planning, retro] });

    const user = userEvent.setup();
    const { relink } = await mount("/app/r/4/t/7/links");

    await user.type(
      screen.getByLabelText("Pull request URL"),
      "https://github.com/acme/api/pull/9",
    );

    // Another client unlinks the retro, so it's offered again.
    relink([pr]);
    await waitFor(() => expect(form).toHaveBeenCalledTimes(2));
    expect(urlField("Pull request URL").value).toBe("https://github.com/acme/api/pull/9");

    await user.click(screen.getByRole("tab", { name: "Event" }));
    expect(await screen.findByRole("option", { name: /Retro/ })).toBeDefined();
  });

  it("keeps the newest events when refreshes answer out of order", async () => {
    const stale = deferred<{ events: (typeof planning)[] }>();
    const fresh = deferred<{ events: (typeof planning)[] }>();

    const form = vi
      .spyOn(actions.work, "linkForm")
      .mockResolvedValueOnce({ events: [planning] })
      .mockReturnValueOnce(stale.promise)
      .mockReturnValueOnce(fresh.promise);

    const user = userEvent.setup();
    const { relink } = await mount("/app/r/4/t/7/links");

    await user.click(screen.getByRole("tab", { name: "Event" }));
    await screen.findByRole("option", { name: /Planning/ });

    relink([pr]);
    relink([]);
    await waitFor(() => expect(form).toHaveBeenCalledTimes(3));

    await act(async () => fresh.resolve({ events: [planning, retro] }));
    await act(async () => stale.resolve({ events: [planning] }));

    expect(screen.getByRole("option", { name: /Retro/ })).toBeDefined();
  });

  it("keeps the events shown when a refresh fails", async () => {
    const form = vi
      .spyOn(actions.work, "linkForm")
      .mockResolvedValueOnce({ events: [planning, retro] })
      .mockRejectedValueOnce(new ActionError("NetworkError", "Offline", {}));

    const user = userEvent.setup();
    const { relink } = await mount("/app/r/4/t/7/links");

    await user.click(screen.getByRole("tab", { name: "Event" }));
    await screen.findByRole("option", { name: /Retro/ });

    relink([pr]);
    await waitFor(() => expect(form).toHaveBeenCalledTimes(2));

    expect(screen.getByRole("option", { name: /Planning/ })).toBeDefined();
    expect(screen.getByRole("option", { name: /Retro/ })).toBeDefined();
    expect(screen.queryByText("Couldn't load this room's events.")).toBeNull();
  });

  it("keeps the event the reader chose through a refresh", async () => {
    const form = vi
      .spyOn(actions.work, "linkForm")
      .mockResolvedValueOnce({ events: [planning, retro] })
      .mockResolvedValueOnce({ events: [planning, retro] });

    const user = userEvent.setup();
    const { relink } = await mount("/app/r/4/t/7/links");

    await user.click(screen.getByRole("tab", { name: "Event" }));
    await user.selectOptions(await screen.findByRole("combobox", { name: "Event" }), "32");

    relink([pr]);
    await waitFor(() => expect(form).toHaveBeenCalledTimes(2));

    // SAFETY: the "combobox" named Event is the <select>.
    const select = screen.getByRole("combobox", { name: "Event" }) as HTMLSelectElement;

    expect(select.value).toBe("32");
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
