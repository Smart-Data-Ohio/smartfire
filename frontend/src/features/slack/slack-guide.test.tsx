import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Link,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SlackPersonal } from "../../gen/SlackPersonal.ts";
import type { SlackRunRow } from "../../gen/SlackRunRow.ts";
import { mutations } from "../../store/store.ts";
import { slack } from "../../sync/admin.ts";
import { PersonalSlackSection } from "./personal-slack-section.tsx";
import {
  LINKED_EMAIL_FACT_TITLE,
  personalGuideProgress,
  shouldOfferSlackPrompt,
} from "./slack-guide.tsx";
import { isSlackPromptDismissed, SlackImportPrompt } from "./slack-import-prompt.tsx";

const VIEWER_ID = 42;

function runRow(overrides: Partial<SlackRunRow>): SlackRunRow {
  return {
    id: 10,
    kind: "personal",
    mode: "dry_run",
    status: "completed",
    startedBy: "Ada",
    createdAt: "2026-10-10T12:00:00Z",
    ...overrides,
  };
}

function personalPage(overrides: Partial<SlackPersonal> = {}): SlackPersonal {
  return {
    teamKnown: true,
    connection: { state: "none" },
    connectPath: "/slack/oauth/start?return_to=%2Fslack%2Fimports",
    runs: [],
    ...overrides,
  };
}

/** Renders the prompt beside routes so navigation and route-hiding can be observed. */
async function renderWithPrompt(initialPath: string): Promise<() => string> {
  const root = createRootRoute({
    component: () => (
      <>
        <SlackImportPrompt />
        <Outlet />
      </>
    ),
  });

  const home = createRoute({ getParentRoute: () => root, path: "/", component: () => <p>Home</p> });

  const settings = createRoute({
    getParentRoute: () => root,
    path: "settings",
    component: () => (
      <>
        <Link to="/">Back to home</Link>
        <Outlet />
      </>
    ),
  });

  const slackSettings = createRoute({
    getParentRoute: () => settings,
    path: "slack",
    component: PersonalSlackSection,
  });

  const slackRun = createRoute({
    getParentRoute: () => settings,
    path: "slack/$runId",
    component: () => <p>Run view</p>,
  });

  const integrations = createRoute({
    getParentRoute: () => settings,
    path: "integrations",
    component: () => <p>Integrations</p>,
  });

  const router = createRouter({
    routeTree: root.addChildren([
      home,
      settings.addChildren([slackSettings, slackRun, integrations]),
    ]),
    basepath: "/app",
    history: createMemoryHistory({ initialEntries: [`/app${initialPath}`] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());

  return () => router.state.location.pathname;
}

beforeEach(() => {
  localStorage.clear();
  mutations.reset();
  mutations.setBoot({
    user: { id: VIEWER_ID, name: "Ada", avatarUrl: "/users/42/avatar" },
    customStyles: null,
    account: {
      name: "Smart Data",
      logoUrl: null,
      logoStillUrl: null,
      bannerUrl: null,
      bannerStillUrl: null,
      uploadLimitBytes: 10_000_000,
    },
    theme: "system",
    textSize: "default",
    cableUrl: "/cable",
    serviceWorkerUrl: null,
    version: "test",
    revision: null,
    flash: null,
    appearancePreferences: null,
  });
});

afterEach(() => {
  cleanup();
  localStorage.clear();
  mutations.reset();
  vi.restoreAllMocks();
});

describe("personal Slack guide progress", () => {
  it("tracks step 1 -> 2 -> 3 -> 4 -> done as a person progresses", () => {
    expect(personalGuideProgress(personalPage()).activeStep).toBe(1);

    expect(
      personalGuideProgress(personalPage({ connection: { state: "connected" } })).activeStep,
    ).toBe(2);

    const afterPreview = personalGuideProgress(
      personalPage({
        connection: { state: "connected" },
        runs: [runRow({ id: 7, mode: "dry_run", status: "completed" })],
      }),
    );

    expect(afterPreview.activeStep).toBe(3);
    expect(afterPreview.latestCompletedPreviewId).toBe(7);

    expect(
      personalGuideProgress(
        personalPage({
          connection: { state: "connected" },
          runs: [runRow({ id: 8, mode: "import", status: "running" })],
        }),
      ).activeStep,
    ).toBe(4);

    expect(
      personalGuideProgress(
        personalPage({
          connection: { state: "connected" },
          runs: [runRow({ id: 9, mode: "import", status: "completed" })],
        }),
      ).activeStep,
    ).toBe("done");
  });

  it("gives active previews, undos, and second imports precedence over completed runs", () => {
    const activePreview = personalGuideProgress(
      personalPage({
        connection: { state: "connected" },
        runs: [
          runRow({ id: 14, mode: "dry_run", status: "running" }),
          runRow({ id: 7, mode: "dry_run", status: "completed" }),
        ],
      }),
    );

    expect(activePreview.activeStep).toBe(2);
    expect(activePreview.activeRun).toEqual({ id: 14, mode: "dry_run", status: "running" });

    const undoing = personalGuideProgress(
      personalPage({
        connection: { state: "connected" },
        runs: [
          runRow({ id: 15, mode: "import", status: "undoing" }),
          runRow({ id: 7, mode: "dry_run", status: "completed" }),
        ],
      }),
    );

    expect(undoing.activeStep).toBe(4);
    expect(undoing.activeRun).toEqual({ id: 15, mode: "import", status: "undoing" });

    const secondImport = personalGuideProgress(
      personalPage({
        connection: { state: "connected" },
        runs: [
          runRow({ id: 16, mode: "import", status: "queued" }),
          runRow({ id: 9, mode: "import", status: "completed" }),
        ],
      }),
    );

    expect(secondImport.activeStep).toBe(4);
    expect(secondImport.activeRun).toEqual({ id: 16, mode: "import", status: "queued" });

    const newerPreview = personalGuideProgress(
      personalPage({
        connection: { state: "connected" },
        runs: [
          runRow({ id: 18, mode: "dry_run", status: "completed" }),
          runRow({ id: 9, mode: "import", status: "completed" }),
        ],
      }),
    );

    expect(newerPreview.activeStep).toBe(3);
    expect(newerPreview.latestCompletedPreviewId).toBe(18);
  });

  it("offers the prompt only when workspace import is ready and no run is active or completed", () => {
    expect(shouldOfferSlackPrompt(personalPage({ teamKnown: false }))).toBe(false);
    expect(shouldOfferSlackPrompt(personalPage())).toBe(true);
    expect(
      shouldOfferSlackPrompt(
        personalPage({ runs: [runRow({ mode: "dry_run", status: "completed" })] }),
      ),
    ).toBe(true);
    expect(
      shouldOfferSlackPrompt(
        personalPage({ runs: [runRow({ mode: "dry_run", status: "running" })] }),
      ),
    ).toBe(false);
    expect(
      shouldOfferSlackPrompt(
        personalPage({ runs: [runRow({ mode: "import", status: "running" })] }),
      ),
    ).toBe(false);
    expect(
      shouldOfferSlackPrompt(
        personalPage({ runs: [runRow({ mode: "import", status: "undoing" })] }),
      ),
    ).toBe(false);
    expect(
      shouldOfferSlackPrompt(
        personalPage({ runs: [runRow({ mode: "import", status: "completed" })] }),
      ),
    ).toBe(false);
    expect(
      shouldOfferSlackPrompt(
        personalPage({ runs: [runRow({ mode: "import", status: "undone" })] }),
      ),
    ).toBe(true);
  });
});

describe("SlackImportPrompt", () => {
  it("shows the guided prompt, opens the plain-English walkthrough dialog, and jumps to import", async () => {
    vi.spyOn(slack, "personal").mockResolvedValue(personalPage());

    const pathname = await renderWithPrompt("/");
    const prompt = await screen.findByRole("region", { name: "Import your Slack history" });

    fireEvent.click(within(prompt).getByRole("button", { name: "How it works" }));

    const dialog = await screen.findByRole("dialog", {
      name: "Import your personal Slack history",
    });

    expect(within(dialog).getByText("Connect your Slack account")).toBeTruthy();
    expect(within(dialog).getByText("Your privacy:")).toBeTruthy();
    expect(
      within(dialog).getByRole("link", { name: "Profile settings" }).getAttribute("href"),
    ).toBe("/app/settings");

    fireEvent.click(within(dialog).getByRole("button", { name: "Go to Slack import" }));

    await waitFor(() => expect(pathname()).toBe("/settings/slack"));
    expect(screen.queryByRole("region", { name: "Import your Slack history" })).toBeNull();
  });

  it("clears cached eligibility on settings entry so a failed refresh does not re-show stale prompt", async () => {
    const personal = vi
      .spyOn(slack, "personal")
      .mockResolvedValueOnce(personalPage())
      .mockResolvedValueOnce(personalPage())
      .mockRejectedValueOnce(new Error("offline"));

    const pathname = await renderWithPrompt("/");
    const prompt = await screen.findByRole("region", { name: "Import your Slack history" });

    fireEvent.click(within(prompt).getByRole("button", { name: "Import from Slack" }));
    await waitFor(() => expect(pathname()).toBe("/settings/slack"));
    await screen.findByRole("heading", { name: "How it works" });

    fireEvent.click(screen.getByRole("link", { name: "Back to home" }));
    await waitFor(() => expect(pathname()).toBe("/"));
    await waitFor(() => expect(personal).toHaveBeenCalledTimes(3));
    expect(screen.queryByRole("region", { name: "Import your Slack history" })).toBeNull();
  });

  it("remembers dismissal for the current user", async () => {
    vi.spyOn(slack, "personal").mockResolvedValue(personalPage());

    await renderWithPrompt("/");

    const prompt = await screen.findByRole("region", { name: "Import your Slack history" });

    fireEvent.click(within(prompt).getByRole("button", { name: "Dismiss Slack import prompt" }));

    expect(screen.queryByRole("region", { name: "Import your Slack history" })).toBeNull();
    expect(isSlackPromptDismissed(VIEWER_ID)).toBe(true);
  });

  it("stays hidden when workspace Slack import is not configured or already imported", async () => {
    const personal = vi
      .spyOn(slack, "personal")
      .mockResolvedValue(personalPage({ teamKnown: false }));

    await renderWithPrompt("/");
    await waitFor(() => expect(personal).toHaveBeenCalledTimes(1));
    expect(screen.queryByRole("region", { name: "Import your Slack history" })).toBeNull();

    cleanup();
    personal.mockResolvedValue(
      personalPage({ runs: [runRow({ mode: "import", status: "completed" })] }),
    );
    await renderWithPrompt("/");
    await waitFor(() => expect(personal).toHaveBeenCalledTimes(2));
    expect(screen.queryByRole("region", { name: "Import your Slack history" })).toBeNull();
  });

  it("does not appear on settings pages", async () => {
    const personal = vi.spyOn(slack, "personal").mockResolvedValue(personalPage());

    await renderWithPrompt("/settings/integrations");
    expect(screen.queryByRole("region", { name: "Import your Slack history" })).toBeNull();
    expect(personal).not.toHaveBeenCalled();
  });
});

describe("PersonalSlackSection walkthrough", () => {
  it("shows the step-by-step guide, Good to know, and a direct link to a finished preview", async () => {
    vi.spyOn(slack, "personal").mockResolvedValue(
      personalPage({
        connection: { state: "connected" },
        runs: [runRow({ id: 12, mode: "dry_run", status: "completed" })],
      }),
    );

    await renderWithPrompt("/settings/slack");

    expect(await screen.findByRole("heading", { name: "How it works" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Good to know" })).toBeTruthy();

    const previewLink = screen.getByRole("link", {
      name: /Review preview #12 and pick conversations/,
    });

    expect(previewLink.getAttribute("href")).toBe("/app/settings/slack/12");
  });

  it("tells someone whose Slack email differs to link it in their profile, and links there", async () => {
    vi.spyOn(slack, "personal").mockResolvedValue(personalPage());

    const pathname = await renderWithPrompt("/settings/slack");
    const fact = (await screen.findByText(`${LINKED_EMAIL_FACT_TITLE}:`)).closest("li");

    expect(fact?.textContent).toContain(
      "If your Slack account uses another email address, add it under Email addresses in Profile settings.",
    );
    expect(fact?.textContent).toContain("moves over any unclaimed placeholder");
    expect(fact?.textContent).toContain("Connecting Slack also moves your unclaimed placeholder");

    const profile = within(fact ?? document.body).getByRole("link", { name: "Profile settings" });

    expect(profile.getAttribute("href")).toBe("/app/settings");
    fireEvent.click(profile);
    await waitFor(() => expect(pathname()).toBe("/settings"));
  });

  it("links to an active preview instead of an older completed preview", async () => {
    vi.spyOn(slack, "personal").mockResolvedValue(
      personalPage({
        connection: { state: "connected" },
        runs: [
          runRow({ id: 21, mode: "dry_run", status: "running" }),
          runRow({ id: 12, mode: "dry_run", status: "completed" }),
        ],
      }),
    );

    await renderWithPrompt("/settings/slack");

    const activeLink = await screen.findByRole("link", {
      name: /Preview #21 is in progress — view status/,
    });

    expect(activeLink.getAttribute("href")).toBe("/app/settings/slack/21");
    expect(screen.getByText(/Your preview is scanning your Slack conversations/)).toBeTruthy();
    expect(screen.queryByRole("link", { name: /Review preview #12/ })).toBeNull();
  });

  it("links to an active undo instead of offering to import a preview", async () => {
    vi.spyOn(slack, "personal").mockResolvedValue(
      personalPage({
        connection: { state: "connected" },
        runs: [
          runRow({ id: 25, mode: "import", status: "undoing" }),
          runRow({ id: 12, mode: "dry_run", status: "completed" }),
        ],
      }),
    );

    await renderWithPrompt("/settings/slack");

    const undoLink = await screen.findByRole("link", {
      name: /Undo #25 is in progress — wait for it to finish/,
    });

    expect(undoLink.getAttribute("href")).toBe("/app/settings/slack/25");
    expect(screen.getByText(/Your undo is running in the background/)).toBeTruthy();
    expect(screen.queryByRole("link", { name: /Review preview #12/ })).toBeNull();
  });

  it("shows the review link for a new preview completed after an earlier import", async () => {
    vi.spyOn(slack, "personal").mockResolvedValue(
      personalPage({
        connection: { state: "connected" },
        runs: [
          runRow({ id: 30, mode: "dry_run", status: "completed" }),
          runRow({ id: 15, mode: "import", status: "completed" }),
        ],
      }),
    );

    await renderWithPrompt("/settings/slack");

    const reviewLink = await screen.findByRole("link", {
      name: /Review preview #30 and pick conversations/,
    });

    expect(reviewLink.getAttribute("href")).toBe("/app/settings/slack/30");
  });
});
