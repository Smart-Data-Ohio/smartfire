import {
  createMemoryHistory,
  createRootRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { OAuthConsent } from "../../gen/OAuthConsent.ts";
import type { PersonalAgent } from "../../gen/PersonalAgent.ts";
import { ActionError } from "../../sync/run.ts";
import { oauth } from "../../sync/settings.ts";
import { pageExit } from "../auth/auth-navigation.ts";
import { ConsentPage } from "./consent-page.tsx";

beforeEach(() => {
  // jsdom has no matchMedia; the agent avatar asks it about the theme and reduced motion.
  window.matchMedia = (query: string) =>
    Object.assign(new EventTarget(), {
      matches: false,
      media: query,
      onchange: null,
      addListener: () => undefined,
      removeListener: () => undefined,
    });
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const CALLBACK = "https://client.example/callback?code=abc&state=xyz";

const HELPER: PersonalAgent = {
  id: 7,
  userId: 70,
  name: "Helper",
  iconName: "owl",
  iconUrl: null,
  provider: null,
  runtime: null,
  description: null,
  dailyMessageCap: 200,
  suspended: false,
  roomIds: [2],
  capabilities: ["post_messages", "read_messages"],
  lastUsedAt: null,
  createdAt: "2026-10-13T12:00:00.000Z",
  credentials: [],
};

function consent(change: Partial<OAuthConsent> = {}): OAuthConsent {
  return {
    clientName: "Example Assistant",
    redirectHostname: "client.example",
    badge: "unverifiedApp",
    agents: [HELPER],
    selectedAgentId: 7,
    rooms: [
      { id: 1, name: "General", private: false },
      { id: 2, name: "Design", private: false },
      { id: 3, name: "Leads", private: true },
    ],
    capabilities: ["read_messages", "post_messages", "react", "manage_threads"],
    enabled: true,
    limit: 3,
    accessAppliesToAllConnections: true,
    roomAccess: [
      {
        agentId: 7,
        roomId: 2,
        roomName: "Design",
        capabilities: ["post_messages", "read_messages"],
      },
    ],
    ...change,
  };
}

/** Renders the page for request `abc`, with the server's answer to reading it. */
async function mount(answer: Promise<OAuthConsent>) {
  const read = vi.spyOn(oauth, "consent").mockReturnValue(answer);
  const leave = vi.spyOn(pageExit, "assign").mockImplementation(() => undefined);
  const root = createRootRoute({ component: () => <ConsentPage publicId="abc" /> });

  const router = createRouter({
    routeTree: root,
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());

  return { read, leave };
}

const approveButton = () => screen.getByRole("button", { name: "Approve" });

describe("the approval page", () => {
  it("names the app, where it goes back to, and that Smartfire hasn't verified it", async () => {
    const { read } = await mount(Promise.resolve(consent()));

    expect(
      await screen.findByRole("heading", {
        name: "Example Assistant wants to connect as one of your agents",
      }),
    ).toBeTruthy();
    expect(read).toHaveBeenCalledWith("abc");
    expect(screen.getByText("client.example")).toBeTruthy();
    expect(screen.getByText(/hasn't verified this app/).closest("[role=note]")).toBeTruthy();
    // The agent this app connected as last time is picked, with its access.
    expect(screen.getByRole("radio", { name: /Helper/ })).toHaveProperty("checked", true);
    expect(screen.getByRole("checkbox", { name: "Design" })).toHaveProperty("checked", true);
    expect(screen.getByRole("checkbox", { name: /Leads/ }).closest("li")?.textContent).toContain(
      "Private",
    );
  });

  it("warns differently about an app on this computer", async () => {
    await mount(Promise.resolve(consent({ badge: "appOnThisComputer" })));

    expect(await screen.findByText(/app on your own computer/)).toBeTruthy();
    expect(screen.queryByText(/hasn't verified this app/)).toBeNull();
  });

  it("shows what approving changes for an existing agent, then approves and goes back", async () => {
    const approve = vi.spyOn(oauth, "approve").mockResolvedValue({ redirectTo: CALLBACK });
    const { leave } = await mount(Promise.resolve(consent()));

    const diff = await screen.findByRole("region", { name: "Changes to Helper's access" });

    expect(within(diff).getByText(/No change/)).toBeTruthy();

    fireEvent.click(screen.getByRole("checkbox", { name: "General" }));
    fireEvent.click(screen.getByRole("checkbox", { name: /Post messages/ }));
    fireEvent.click(screen.getByRole("checkbox", { name: /React/ }));

    expect(within(diff).getByText("Joins").parentElement?.textContent).toBe(
      "Joins General: can read messages, react",
    );
    expect(within(diff).getByText("In").parentElement?.textContent).toBe(
      "In Design: can now react; can no longer post messages",
    );

    fireEvent.click(screen.getByRole("checkbox", { name: "Design" }));
    expect(within(diff).getByText("Leaves").parentElement?.textContent).toBe(
      "Leaves Design: can no longer read messages, post messages",
    );

    fireEvent.click(approveButton());

    await waitFor(() => expect(leave).toHaveBeenCalledWith(CALLBACK));
    expect(approve).toHaveBeenCalledWith("abc", {
      agentId: 7,
      name: null,
      iconName: null,
      roomIds: [1],
      capabilities: ["read_messages", "react"],
    });
    expect(await screen.findByText("Taking you back to the app…")).toBeTruthy();
  });

  it("compares each room, so posting in one room isn't shown as no change for another", async () => {
    const approve = vi.spyOn(oauth, "approve").mockResolvedValue({ redirectTo: CALLBACK });
    // Helper reads General and Design but posts only in General.
    const reader = { ...HELPER, roomIds: [1, 2] };

    const { leave } = await mount(
      Promise.resolve(
        consent({
          agents: [reader],
          roomAccess: [
            {
              agentId: 7,
              roomId: 1,
              roomName: "General",
              capabilities: ["post_messages", "read_messages"],
            },
            { agentId: 7, roomId: 2, roomName: "Design", capabilities: ["read_messages"] },
          ],
        }),
      ),
    );

    const diff = await screen.findByRole("region", { name: "Changes to Helper's access" });

    // The untouched form gives every chosen room every chosen ability: Design gains posting.
    expect(within(diff).queryByText(/No change/)).toBeNull();
    expect(
      within(diff)
        .getAllByRole("listitem")
        .map((item) => item.textContent),
    ).toEqual(["In Design: can now post messages"]);

    fireEvent.click(approveButton());

    await waitFor(() => expect(leave).toHaveBeenCalledWith(CALLBACK));
    expect(approve).toHaveBeenCalledWith("abc", {
      agentId: 7,
      name: null,
      iconName: null,
      roomIds: [1, 2],
      capabilities: ["post_messages", "read_messages"],
    });
  });

  it("names a room the person has left, which approval takes the agent out of", async () => {
    await mount(
      Promise.resolve(
        consent({
          roomAccess: [
            {
              agentId: 7,
              roomId: 2,
              roomName: "Design",
              capabilities: ["post_messages", "read_messages"],
            },
            { agentId: 7, roomId: 9, roomName: "Old project", capabilities: [] },
          ],
        }),
      ),
    );

    const diff = await screen.findByRole("region", { name: "Changes to Helper's access" });

    expect(
      within(diff)
        .getAllByRole("listitem")
        .map((item) => item.textContent),
    ).toEqual(["Leaves Old project: it couldn't do anything there"]);
  });

  it("makes a new agent, which needs a name", async () => {
    const approve = vi.spyOn(oauth, "approve").mockResolvedValue({ redirectTo: CALLBACK });
    await mount(Promise.resolve(consent({ selectedAgentId: null })));

    // With no agent connected before, a new one is picked while the limit allows.
    const fresh = await screen.findByRole("radio", { name: /A new agent/ });

    expect(fresh).toHaveProperty("checked", true);
    expect(screen.queryByRole("region", { name: /Changes to/ })).toBeNull();

    fireEvent.click(approveButton());
    expect((await screen.findByRole("alert")).textContent).toBe("Give the agent a name.");
    expect(approve).not.toHaveBeenCalled();

    fireEvent.change(screen.getByRole("textbox", { name: /Name/ }), {
      target: { value: "  Assistant " },
    });
    fireEvent.click(approveButton());

    await waitFor(() =>
      expect(approve).toHaveBeenCalledWith("abc", {
        agentId: null,
        name: "Assistant",
        iconName: null,
        roomIds: [],
        capabilities: [],
      }),
    );
  });

  it("offers no new agent at the limit", async () => {
    await mount(Promise.resolve(consent({ selectedAgentId: null, limit: 1 })));

    expect(await screen.findByRole("radio", { name: /Helper/ })).toHaveProperty("checked", true);
    expect(screen.queryByRole("radio", { name: /A new agent/ })).toBeNull();
    expect(screen.getByText(/as many agents as this workspace allows \(1\)/)).toBeTruthy();
  });

  it("stays put when the confirmation is closed, and approves once confirmed", async () => {
    const approve = vi
      .spyOn(oauth, "approve")
      .mockRejectedValueOnce(new ActionError("ConfirmationCancelled", "Cancelled"))
      .mockResolvedValueOnce({ redirectTo: CALLBACK });

    const { leave } = await mount(Promise.resolve(consent()));

    fireEvent.click(await screen.findByRole("button", { name: "Approve" }));

    await waitFor(() => expect(approveButton()).toHaveProperty("disabled", false));
    expect(leave).not.toHaveBeenCalled();
    expect(screen.queryByRole("alert")).toBeNull();

    fireEvent.click(approveButton());

    await waitFor(() => expect(leave).toHaveBeenCalledWith(CALLBACK));
    expect(approve).toHaveBeenCalledTimes(2);
  });

  it("denies, going back to the app with its refusal", async () => {
    const refused = "https://client.example/callback?error=access_denied&state=xyz";
    const deny = vi.spyOn(oauth, "deny").mockResolvedValue({ redirectTo: refused });
    const { leave } = await mount(Promise.resolve(consent()));

    fireEvent.click(await screen.findByRole("button", { name: "Deny" }));

    await waitFor(() => expect(leave).toHaveBeenCalledWith(refused));
    expect(deny).toHaveBeenCalledWith("abc");
  });

  it("says plainly when the request expired, was used or isn't yours", async () => {
    const { leave } = await mount(Promise.reject(new ActionError("NotFound", "Not found")));

    expect(await screen.findByRole("heading", { name: "This request has ended" })).toBeTruthy();
    expect(screen.getByText(/Requests last 10 minutes/)).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Approve" })).toBeNull();
    expect(leave).not.toHaveBeenCalled();
  });

  it("says so too when it ends while the page is open", async () => {
    vi.spyOn(oauth, "approve").mockRejectedValue(new ActionError("NotFound", "Not found"));
    await mount(Promise.resolve(consent()));

    fireEvent.click(await screen.findByRole("button", { name: "Approve" }));

    expect(await screen.findByRole("heading", { name: "This request has ended" })).toBeTruthy();
  });

  it("only lets the person deny while personal agents are off", async () => {
    await mount(Promise.resolve(consent({ enabled: false })));

    expect(await screen.findByText(/Personal agents are turned off/)).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Approve" })).toBeNull();
    expect(screen.getByRole("button", { name: "Deny" })).toBeTruthy();
  });
});
