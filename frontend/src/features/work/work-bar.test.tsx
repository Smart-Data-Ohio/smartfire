import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterAll, beforeAll, beforeEach, describe, expect, it } from "vitest";
import { SEED_IDS } from "../../../mock/server.ts";
import type { Me } from "../../gen/Me.ts";
import type { ThreadDetail } from "../../gen/ThreadDetail.ts";
import { mutations, store } from "../../store/store.ts";
import { installMockNetwork, type MockNetwork } from "../../test/mock-network.ts";
import {
  factsFixture,
  threadDetailFixture,
  workDetailFixture,
  workPermissionsFixture,
} from "./test-fixtures.ts";
import { WorkBar, WorkLive } from "./work-bar.tsx";

// The real actions run against the in-memory mock backend through stubbed fetch.
let network: MockNetwork;

const WORK = SEED_IDS.s4.work;

/** Loads a thread's detail into the store, as opening its pane does. */
async function load(threadId: number): Promise<ThreadDetail> {
  const detail: ThreadDetail = await (await fetch(`/api/v1/threads/${threadId}`)).json();

  act(() => mutations.loadThreadDetail(detail));

  return detail;
}

/** The bar reads the handoff dialog off the thread's child route, so tests sit in that router. */
async function renderBar(threadId: number, live = true) {
  const roomId = store.getState().threads[threadId]?.roomId ?? 1;
  const root = createRootRoute({ component: Outlet });

  const thread = createRoute({
    getParentRoute: () => root,
    path: "/r/$roomId/t/$threadId",
    component: () => (
      <>
        <div className="pane-frame">
          <button type="button" aria-label="Thread actions" />
          <WorkBar threadId={threadId} />
          {live ? <WorkLive threadId={threadId} /> : null}
        </div>
        <Outlet />
      </>
    ),
  });

  const handoff = createRoute({
    getParentRoute: () => thread,
    path: "handoff",
    component: () => null,
  });

  const router = createRouter({
    routeTree: root.addChildren([thread.addChildren([handoff])]),
    history: createMemoryHistory({ initialEntries: [`/r/${roomId}/t/${threadId}`] }),
  });

  const view = render(<RouterProvider router={router} />);

  await act(() => router.load());

  return view;
}

const factsOf = (threadId: number) => store.getState().threads[threadId]?.work ?? null;

/** What the live regions have said. */
const announced = () =>
  screen
    .getAllByRole("status")
    .map((region) => region.textContent)
    .join(" ");

beforeAll(() => {
  network = installMockNetwork();

  const meta = document.createElement("meta");

  meta.name = "csrf-token";
  meta.content = network.server.csrfToken();
  document.head.append(meta);
  Element.prototype.scrollIntoView = () => undefined;
});

afterAll(() => network.restore());

beforeEach(async () => {
  // jsdom has no matchMedia; the motion helpers ask it about reduced motion.
  window.matchMedia = (query: string) =>
    Object.assign(new EventTarget(), {
      matches: false,
      media: query,
      onchange: null,
      addListener: () => undefined,
      removeListener: () => undefined,
    });

  await fetch("/__mock/reset", { method: "POST" });

  const me: Me = await (await fetch("/api/v1/me")).json();

  mutations.reset();
  mutations.setMe(me);
});

describe("the thread pane's work section", () => {
  it("hides stop tracking for a board post while keeping its status menu", async () => {
    const detail = threadDetailFixture(
      901,
      factsFixture(),
      workDetailFixture(),
      workPermissionsFixture({ canRemoveWork: false }),
    );

    act(() => mutations.loadThreadDetail({ ...detail, thread: { ...detail.thread, roomId: 41 } }));
    await renderBar(901, false);

    await userEvent.click(screen.getByRole("button", { name: /Change status/ }));

    expect(screen.getByRole("menuitemradio", { name: "Done" })).toBeTruthy();
    expect(screen.queryByRole("menuitem", { name: "Stop tracking…" })).toBeNull();
  });

  it("shows the status, owner, links and run, and changes the status from its menu", async () => {
    await load(WORK.agentOwned);
    await renderBar(WORK.agentOwned);

    const user = userEvent.setup();
    const links = screen.getByRole("list", { name: /^Links for/ });

    expect(within(links).getByRole("link", { name: "Run, opens in a new tab" })).toBeTruthy();
    expect(within(links).getByRole("link", { name: /^Pull request, .*, Open$/ })).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "Status: In progress. Change status" }));
    await user.click(screen.getByRole("menuitemradio", { name: "Blocked" }));

    await waitFor(() => expect(factsOf(WORK.agentOwned)?.status).toBe("blocked"));
    expect(announced()).toContain("Status: Blocked");
    expect(screen.getByRole("button", { name: /^Owner: Ember\. Change owner$/ })).toBeTruthy();
  });

  it("offers people and agents as owners, with an agent's provider and description", async () => {
    await load(WORK.agentOwned);
    await renderBar(WORK.agentOwned);

    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "Owner: Ember. Change owner" }));

    const agents = screen.getByRole("group", { name: "Agents" });
    const people = screen.getByRole("group", { name: "People" });

    expect(within(agents).getByText(/Anthropic · /)).toBeTruthy();
    expect(
      within(agents)
        .getByRole("menuitemradio", { name: /^Ember/ })
        .getAttribute("aria-checked"),
    ).toBe("true");

    await user.click(within(people).getByRole("menuitemradio", { name: /^Maya/ }));

    await waitFor(() => expect(factsOf(WORK.agentOwned)?.owner?.name).toMatch(/^Maya/));
    expect(announced()).toMatch(/Owner: Maya/);
  });

  it("shows only facts when the viewer may change nothing", async () => {
    await load(WORK.agentOwned);

    const permissions = workPermissionsFixture({
      canManageWork: false,
      canUpdateWorkStatus: false,
      canAssignWork: false,
      canRemoveWork: false,
    });

    act(() =>
      store.setState((state) => ({
        ...state,
        threadPanes: {
          ...state.threadPanes,
          [WORK.agentOwned]: {
            status: "ready",
            error: null,
            permissions,
            work: null,
            workFacts: null,
          },
        },
      })),
    );
    await renderBar(WORK.agentOwned);

    expect(screen.queryByRole("button", { name: /Change status/ })).toBeNull();
    expect(screen.queryByRole("button", { name: /Change owner/ })).toBeNull();
    expect(screen.queryByRole("button", { name: /^(Add|Edit) result$/ })).toBeNull();
    expect(screen.queryByRole("button", { name: "Hand off to an agent" })).toBeNull();
    expect(document.querySelector<HTMLElement>(".work-status")?.dataset.status).toBe("in_progress");
  });

  it("records a result and says who updated it", async () => {
    await load(WORK.viewerOwned);
    await renderBar(WORK.viewerOwned);

    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "Result, steps and history" }));
    await user.click(screen.getByRole("button", { name: "Add result" }));
    await user.type(screen.getByRole("textbox", { name: "Result" }), "Shipped the **new** copy");
    await user.click(screen.getByRole("button", { name: "Save result" }));

    await waitFor(() =>
      expect(document.querySelector(".work-result-body strong")?.textContent).toBe("new"),
    );
    expect(screen.getByText(/^Updated/).textContent).toMatch(/by /);
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Edit result" }));
  });

  it("checks a handoff as the server does, then hands the work to the agent", async () => {
    await load(WORK.done);
    await renderBar(WORK.done);

    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "Result, steps and history" }));
    await user.click(screen.getByRole("button", { name: "Hand off to an agent" }));

    const dialog = await screen.findByRole("dialog");

    await user.type(within(dialog).getByRole("textbox", { name: /^Links/ }), "ftp://nope");
    await user.click(within(dialog).getByRole("button", { name: "Hand off" }));

    expect(within(dialog).getByText("Summary can't be blank")).toBeTruthy();
    expect(within(dialog).getByText("Links must be http(s) URLs")).toBeTruthy();

    await user.clear(within(dialog).getByRole("textbox", { name: /^Links/ }));
    await user.type(within(dialog).getByRole("textbox", { name: "Summary" }), "Copy is drafted");
    await user.click(within(dialog).getByRole("radio", { name: /Ember/ }));
    await user.click(within(dialog).getByRole("button", { name: "Hand off" }));

    await waitFor(() => expect(factsOf(WORK.done)?.owner?.name).toBe("Ember"));
    expect(store.getState().work.details[WORK.done]?.history[0]?.kind).toBe("handoff");
  });

  it("stops tracking after a confirmation, and focus lands on the thread's actions", async () => {
    await load(WORK.unassigned);

    const view = await renderBar(WORK.unassigned);
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: /Change status/ }));
    await user.click(screen.getByRole("menuitem", { name: "Stop tracking…" }));

    const confirm = await screen.findByRole("alertdialog");

    await user.click(within(confirm).getByRole("button", { name: "Stop tracking" }));

    await waitFor(() => expect(factsOf(WORK.unassigned)).toBeNull());
    expect(view.container.querySelector(".work-bar")).toBeNull();
    await waitFor(() =>
      expect(document.activeElement).toBe(screen.getByRole("button", { name: "Thread actions" })),
    );
    expect(announced()).toContain("No longer tracked as work");
  });

  it("names a history entry with no actor a former member", async () => {
    await load(WORK.done);
    await renderBar(WORK.done);

    expect(screen.getAllByText("Former member").length).toBeGreaterThan(0);
  });

  it("refetches the detail when someone else changes the work", async () => {
    await load(WORK.agentOwned);
    await renderBar(WORK.agentOwned);

    const before = store.getState().work.details[WORK.agentOwned]?.history.length ?? 0;
    const detail: ThreadDetail = await (await fetch(`/api/v1/threads/${WORK.agentOwned}`)).json();

    // Someone else moves it to Done: the server's facts arrive as thread.updated would bring them.
    await fetch("/__mock/work-status", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ threadId: WORK.agentOwned, status: "done" }),
    });

    const moved: ThreadDetail = await (await fetch(`/api/v1/threads/${WORK.agentOwned}`)).json();

    expect(moved.thread.work?.status).toBe("done");
    act(() => mutations.upsertThread({ ...detail.thread, work: moved.thread.work }));

    await waitFor(() =>
      expect(store.getState().work.details[WORK.agentOwned]?.history.length).toBe(before + 1),
    );
    expect(announced()).toContain("Status: Done");
  });
});
