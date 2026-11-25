import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { meFixture, messageFixture, sidebarRowFixture, userFixture } from "../../api/testing.ts";
import type { PersonProfile } from "../../gen/PersonProfile.ts";
import type { User } from "../../gen/User.ts";
import type { UserRole } from "../../gen/UserRole.ts";
import { initialState } from "../../store/state.ts";
import { mutations, store } from "../../store/store.ts";
import { peoplePages } from "../../sync/admin.ts";
import { directs } from "../../sync/directs.ts";
import { BodyHtml } from "../messages/body-html.tsx";
import { MessageRow } from "../room/message-row.tsx";
import { HitRow } from "../search/hit-row.tsx";
import { clearUserCardProfiles, UserCardHost } from "./user-card.tsx";
import { OPEN_DELAY_MS, resetUserCard } from "./user-card-store.ts";
import { UserCardTrigger } from "./user-card-trigger.tsx";

const PERSON = 8;

const VIEWER = 1;

function person(patch: Partial<User> = {}): User {
  return { ...userFixture(PERSON, "Ashley"), accountName: "Ashley Brown", ...patch };
}

function profileOf(user: User, patch: Partial<PersonProfile> = {}): PersonProfile {
  return {
    user,
    ignoredByMe: false,
    status: null,
    dndAllowed: null,
    emailAddress: null,
    transferUrl: null,
    transferQrSvg: null,
    canBan: false,
    canManageBot: false,
    ...patch,
  };
}

/** Signs the viewer in with `role`, or changes their role. */
function signInAs(role: UserRole): void {
  act(() =>
    store.setState({ me: { ...meFixture, user: { ...userFixture(VIEWER, "Viewer"), role } } }),
  );
}

/** Lets a hover's cold delay run out, so a card it would open has opened. */
async function afterHoverDelay(): Promise<void> {
  await act(() => new Promise((resolve) => setTimeout(resolve, OPEN_DELAY_MS + 50)));
}

/** Renders `content` beside the card host, under routes the card's links and Message go to. */
async function renderWithCard(content: ReactNode): Promise<() => string> {
  const root = createRootRoute({
    component: () => (
      <>
        {content}
        <UserCardHost />
      </>
    ),
  });

  const page = (path: string) =>
    createRoute({ getParentRoute: () => root, path, component: () => null });

  const router = createRouter({
    routeTree: root.addChildren([
      page("/"),
      page("r/$roomId"),
      page("r/$roomId/m/$messageId"),
      page("people/$userId"),
      page("agents/$agentId"),
      page("settings"),
    ]),
    basepath: "/app",
    history: createMemoryHistory({ initialEntries: ["/app/"] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());

  return () => router.state.location.pathname;
}

async function openCard(name: string): Promise<HTMLElement> {
  fireEvent.click(screen.getByRole("button", { name }));

  return screen.findByRole("dialog");
}

async function expectClosed(): Promise<void> {
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
}

beforeEach(() => {
  // jsdom has no matchMedia; a bot's generated avatar asks it for the theme.
  window.matchMedia = (query: string) =>
    Object.assign(new EventTarget(), {
      matches: false,
      media: query,
      onchange: null,
      addListener: () => undefined,
      removeListener: () => undefined,
    });

  vi.spyOn(peoplePages, "profile").mockImplementation((userId) => {
    const user = store.getState().users[userId];

    return user === undefined
      ? Promise.reject(new Error("not found"))
      : Promise.resolve(profileOf(user));
  });
});

afterEach(() => {
  vi.restoreAllMocks();
  resetUserCard();
  clearUserCardProfiles();
  store.setState(initialState, true);
});

describe("the user card", () => {
  it("shows their full name, account name, pronouns, presence, custom status and bio", async () => {
    mutations.mergeUsers([
      person({
        pronouns: "she/her",
        bio: "Builds the release train.",
        customStatus: { emoji: "🌴", text: "On leave", expiresAt: null },
      }),
    ]);
    mutations.setPresence([{ userId: PERSON, presence: "online", statusText: null }]);
    await renderWithCard(<UserCardTrigger userId={PERSON}>Ashley</UserCardTrigger>);

    const card = await openCard("Ashley");

    expect(within(card).getByRole("heading", { name: "Ashley" })).toBeTruthy();
    expect(card.getAttribute("aria-labelledby")).toBe(within(card).getByRole("heading").id);
    expect(within(card).getByText("Ashley Brown")).toBeTruthy();
    expect(within(card).getByText("she/her")).toBeTruthy();
    expect(within(card).getByText("Active")).toBeTruthy();
    expect(within(card).getByText("🌴 On leave")).toBeTruthy();
    expect(within(card).getByText("Builds the release train.")).toBeTruthy();
    expect(within(card).getByRole("button", { name: "Message" })).toBeTruthy();
    expect(within(card).getByRole("link", { name: "View profile" }).getAttribute("href")).toBe(
      `/app/people/${PERSON}`,
    );
  });

  it("leaves out the account name when it is their display name", async () => {
    mutations.mergeUsers([person({ accountName: "Ashley" })]);
    await renderWithCard(<UserCardTrigger userId={PERSON}>Ashley</UserCardTrigger>);

    const card = await openCard("Ashley");

    expect(within(card).getAllByText("Ashley")).toHaveLength(1);
  });

  it("adds the email an administrator sees once the profile loads, and the Admin badge", async () => {
    const user = person({ role: "administrator" });

    signInAs("administrator");
    mutations.mergeUsers([user]);
    vi.mocked(peoplePages.profile).mockResolvedValue(
      profileOf(user, { emailAddress: "ashley@example.com" }),
    );
    await renderWithCard(<UserCardTrigger userId={PERSON}>Ashley</UserCardTrigger>);

    const card = await openCard("Ashley");

    expect(within(card).getByText("Admin")).toBeTruthy();

    const email = await within(card).findByRole("link", { name: "ashley@example.com" });

    expect(email.getAttribute("href")).toBe("mailto:ashley@example.com");
    expect(peoplePages.profile).toHaveBeenCalledWith(PERSON, expect.any(AbortSignal));
  });

  it("badges an agent and shows what it is doing instead of presence", async () => {
    mutations.mergeUsers([
      person({
        role: "bot",
        agent: { agentId: 3, kind: "workspace", status: "idle", suspended: false },
      }),
    ]);
    await renderWithCard(<UserCardTrigger userId={PERSON}>Ashley</UserCardTrigger>);

    const card = await openCard("Ashley");

    expect(card.querySelector(".user-card-badges .agent-badge")).not.toBeNull();
    expect(card.querySelector(".user-card-agent")?.textContent).toContain("·");
    expect(within(card).queryByText("Active")).toBeNull();
    expect(within(card).getByRole("link", { name: "View profile" }).getAttribute("href")).toBe(
      "/app/agents/3",
    );
  });

  it("offers no Message button for a deactivated account", async () => {
    mutations.mergeUsers([person({ status: "deactivated", bio: "Gone fishing." })]);
    await renderWithCard(<UserCardTrigger userId={PERSON}>Ashley</UserCardTrigger>);

    const card = await openCard("Ashley");

    expect(within(card).getByText("Deactivated")).toBeTruthy();
    expect(within(card).getByText("No longer on this account")).toBeTruthy();
    expect(within(card).queryByRole("button", { name: "Message" })).toBeNull();
    expect(within(card).queryByText("Gone fishing.")).toBeNull();
  });

  it("opens their direct message from Message and closes", async () => {
    mutations.mergeUsers([person()]);

    const create = vi
      .spyOn(directs, "create")
      .mockResolvedValue(sidebarRowFixture(55, "Ashley", "direct", [PERSON]));

    const pathname = await renderWithCard(
      <UserCardTrigger userId={PERSON}>Ashley</UserCardTrigger>,
    );

    const card = await openCard("Ashley");

    fireEvent.click(within(card).getByRole("button", { name: "Message" }));

    expect(create).toHaveBeenCalledWith([PERSON]);
    await waitFor(() => expect(pathname()).toBe("/r/55"));
    await expectClosed();
  });

  it("closes on Esc, giving focus back to the trigger, and on a press outside", async () => {
    mutations.mergeUsers([person()]);
    await renderWithCard(
      <>
        <UserCardTrigger userId={PERSON}>Ashley</UserCardTrigger>
        <p>Elsewhere</p>
      </>,
    );

    const card = await openCard("Ashley");

    expect(document.activeElement).toBe(within(card).getByRole("button", { name: "Message" }));
    fireEvent.keyDown(document.activeElement ?? card, { key: "Escape" });
    await expectClosed();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Ashley" }));

    await openCard("Ashley");
    fireEvent.pointerDown(screen.getByText("Elsewhere"));
    await expectClosed();

    await openCard("Ashley");
    fireEvent.keyDown(document.body, { key: "Escape" });
    await expectClosed();
  });

  it("toggles closed from its own trigger", async () => {
    mutations.mergeUsers([person()]);
    await renderWithCard(<UserCardTrigger userId={PERSON}>Ashley</UserCardTrigger>);

    await openCard("Ashley");
    fireEvent.pointerDown(screen.getByRole("button", { name: "Ashley" }));
    fireEvent.click(screen.getByRole("button", { name: "Ashley" }));
    await expectClosed();
  });

  it("hides an email it fetched once the viewer is no longer an administrator", async () => {
    const user = person();

    signInAs("administrator");
    mutations.mergeUsers([user]);
    vi.mocked(peoplePages.profile).mockResolvedValue(
      profileOf(user, { emailAddress: "ashley@example.com" }),
    );
    await renderWithCard(<UserCardTrigger userId={PERSON}>Ashley</UserCardTrigger>);

    const card = await openCard("Ashley");

    await within(card).findByRole("link", { name: "ashley@example.com" });
    signInAs("member");
    expect(within(card).queryByRole("link", { name: "ashley@example.com" })).toBeNull();
  });

  it("hides an email it fetched once the account is deactivated", async () => {
    const user = person();

    signInAs("administrator");
    mutations.mergeUsers([user]);
    vi.mocked(peoplePages.profile).mockResolvedValue(
      profileOf(user, { emailAddress: "ashley@example.com" }),
    );
    await renderWithCard(<UserCardTrigger userId={PERSON}>Ashley</UserCardTrigger>);

    const card = await openCard("Ashley");

    await within(card).findByRole("link", { name: "ashley@example.com" });
    act(() =>
      mutations.mergeUsers([
        person({ status: "deactivated", updatedAt: "2026-02-01T00:00:00.000000Z" }),
      ]),
    );
    expect(within(card).getByText("Deactivated")).toBeTruthy();
    expect(within(card).queryByRole("link", { name: "ashley@example.com" })).toBeNull();
  });

  it("stays put when it is dismissed while Message is on its way", async () => {
    mutations.mergeUsers([person()]);

    let land: (row: ReturnType<typeof sidebarRowFixture>) => void = () => undefined;

    vi.spyOn(directs, "create").mockReturnValue(
      new Promise((resolve) => {
        land = resolve;
      }),
    );

    const pathname = await renderWithCard(
      <UserCardTrigger userId={PERSON}>Ashley</UserCardTrigger>,
    );

    const card = await openCard("Ashley");

    fireEvent.click(within(card).getByRole("button", { name: "Message" }));
    fireEvent.keyDown(document.body, { key: "Escape" });
    await act(async () => land(sidebarRowFixture(55, "Ashley", "direct", [PERSON])));
    await expectClosed();
    expect(pathname()).toBe("/");
  });

  it("closes once focus tabs on past a card opened by a click", async () => {
    mutations.mergeUsers([person()]);
    await renderWithCard(
      <>
        <UserCardTrigger userId={PERSON}>Ashley</UserCardTrigger>
        <button type="button">Next</button>
      </>,
    );

    const card = await openCard("Ashley");
    const inside = within(card).getByRole("button", { name: "Message" });

    // Focus moving within the card, or to nothing at all, keeps it open.
    fireEvent.blur(inside, {
      relatedTarget: within(card).getByRole("link", { name: "View profile" }),
    });
    fireEvent.blur(inside, { relatedTarget: null });
    expect(screen.getByRole("dialog")).toBe(card);

    fireEvent.blur(inside, { relatedTarget: screen.getByRole("button", { name: "Next" }) });
    await expectClosed();
  });

  it("gives focus back to the trigger on Esc pressed outside a card opened by a click", async () => {
    mutations.mergeUsers([person()]);
    await renderWithCard(<UserCardTrigger userId={PERSON}>Ashley</UserCardTrigger>);

    await openCard("Ashley");
    fireEvent.keyDown(document.body, { key: "Escape" });
    await expectClosed();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Ashley" }));
  });

  it("marks only the trigger it hangs off as expanded", async () => {
    mutations.mergeUsers([person()]);
    await renderWithCard(
      <>
        <UserCardTrigger userId={PERSON}>Ashley</UserCardTrigger>
        <UserCardTrigger userId={PERSON}>Ashley again</UserCardTrigger>
      </>,
    );

    const first = screen.getByRole("button", { name: "Ashley" });
    const second = screen.getByRole("button", { name: "Ashley again" });

    expect([first, second].map((trigger) => trigger.getAttribute("aria-expanded"))).toEqual([
      "false",
      "false",
    ]);

    await openCard("Ashley");
    expect([first, second].map((trigger) => trigger.getAttribute("aria-expanded"))).toEqual([
      "true",
      "false",
    ]);

    fireEvent.keyDown(document.body, { key: "Escape" });
    await expectClosed();
    expect([first, second].map((trigger) => trigger.getAttribute("aria-expanded"))).toEqual([
      "false",
      "false",
    ]);
  });

  it("opens after a mouse rests on a trigger, never for a touch", async () => {
    mutations.mergeUsers([person()]);
    await renderWithCard(<UserCardTrigger userId={PERSON}>Ashley</UserCardTrigger>);

    const trigger = screen.getByRole("button", { name: "Ashley" });

    fireEvent.pointerEnter(trigger, { pointerType: "touch" });
    await new Promise((resolve) => setTimeout(resolve, OPEN_DELAY_MS + 50));
    expect(screen.queryByRole("dialog")).toBeNull();

    fireEvent.pointerEnter(trigger, { pointerType: "mouse" });
    expect(await screen.findByRole("dialog", {}, { timeout: OPEN_DELAY_MS * 3 })).toBeTruthy();
  });
});

describe("where the card opens from", () => {
  it("opens from a message's author name and avatar", async () => {
    mutations.mergeUsers([userFixture(7, "Priya Patel")]);
    await renderWithCard(
      <MessageRow
        message={messageFixture(41, 1, { creatorId: 7 })}
        groupStart
        mentionsMe={false}
        focused={false}
        live={false}
      />,
    );

    const triggers = document.querySelectorAll<HTMLElement>('[data-user-card-trigger="7"]');

    expect(triggers).toHaveLength(2);

    const avatar = [...triggers].find((trigger) => trigger.getAttribute("aria-hidden") === "true");

    expect(avatar?.tabIndex).toBe(-1);

    const card = await openCard("Priya Patel");

    expect(within(card).getByRole("heading", { name: "Priya Patel" })).toBeTruthy();
    fireEvent.keyDown(document.body, { key: "Escape" });
    await expectClosed();

    fireEvent.click(avatar ?? document.body);
    expect(await screen.findByRole("dialog")).toBeTruthy();
  });

  it("never puts focus on the hidden avatar beside a name, opening or closing", async () => {
    mutations.mergeUsers([userFixture(7, "Priya Patel")]);
    await renderWithCard(
      <MessageRow
        message={messageFixture(41, 1, { creatorId: 7 })}
        groupStart
        mentionsMe={false}
        focused={false}
        live={false}
      />,
    );

    const avatar = document.querySelector<HTMLElement>(
      '[data-user-card-trigger="7"][aria-hidden="true"]',
    );

    expect(avatar).not.toBeNull();
    expect(avatar?.hasAttribute("aria-expanded")).toBe(false);

    // A mouse press on it would focus it; the trigger keeps focus where it was.
    expect(fireEvent.mouseDown(avatar ?? document.body)).toBe(false);
    fireEvent.click(avatar ?? document.body);

    const card = await screen.findByRole("dialog");

    fireEvent.keyDown(within(card).getByRole("button", { name: "Message" }), { key: "Escape" });
    await expectClosed();
    expect(document.activeElement).not.toBe(avatar);
  });

  it("opens a search result's card from its avatar and author's name, not the rest of it", async () => {
    mutations.mergeUsers([userFixture(7, "Priya Patel")]);
    await renderWithCard(
      <HitRow
        hit={messageFixture(41, 1, { creatorId: 7, bodyHtml: "<p>Ship the release</p>" })}
        conversation={undefined}
        terms={[]}
        now={Date.parse("2026-10-06T16:00:00.000Z")}
      />,
    );

    const link = document.querySelector<HTMLElement>("[data-search-hit-link]");
    const name = document.querySelector<HTMLElement>(".search-hit-author");

    // The stretched link covers the card: a pointer over its body or its link alone opens nothing.
    fireEvent.pointerEnter(screen.getByText("Ship the release"), { pointerType: "mouse" });
    fireEvent.pointerEnter(link ?? document.body, { pointerType: "mouse" });
    await afterHoverDelay();
    expect(screen.queryByRole("dialog")).toBeNull();

    fireEvent.pointerEnter(name ?? document.body, { pointerType: "mouse" });

    const card = await screen.findByRole("dialog", {}, { timeout: OPEN_DELAY_MS * 3 });

    expect(within(card).getByRole("heading", { name: "Priya Patel" })).toBeTruthy();
    fireEvent.keyDown(document.body, { key: "Escape" });
    await expectClosed();

    fireEvent.click(
      document.querySelector('.search-hit > [data-user-card-trigger="7"]') ?? document.body,
    );
    expect(await screen.findByRole("dialog")).toBeTruthy();
  });

  it("keeps an agent author's name a link to its profile, with the card on hover", async () => {
    mutations.mergeUsers([
      {
        ...userFixture(7, "Deploy Agent"),
        role: "bot",
        agent: { agentId: 3, kind: "workspace", status: "idle", suspended: false },
      },
    ]);

    const pathname = await renderWithCard(
      <MessageRow
        message={messageFixture(41, 1, { creatorId: 7 })}
        groupStart
        mentionsMe={false}
        focused={false}
        live={false}
      />,
    );

    expect(document.querySelector("[data-user-card-trigger]")).toBeNull();

    const link = screen.getByRole("link", { name: /Deploy Agent/ });

    fireEvent.pointerEnter(link, { pointerType: "mouse" });
    expect(await screen.findByRole("dialog", {}, { timeout: OPEN_DELAY_MS * 3 })).toBeTruthy();

    fireEvent.click(link);
    await waitFor(() => expect(pathname()).toBe("/agents/3"));
  });

  it("opens from a mention in message HTML, but not one a covered spoiler hides", async () => {
    mutations.mergeUsers([userFixture(7, "Maya Okafor"), userFixture(9, "Hidden Person")]);

    const html =
      '<p>Ship it, <span class="mention" data-user-id="7"><span class="profile-card-name">' +
      'Maya Okafor</span></span> and <span class="spoiler" data-spoiler="">' +
      '<span class="mention" data-user-id="9"><span class="profile-card-name">Hidden Person' +
      "</span></span></span></p>";

    await renderWithCard(<BodyHtml html={html} className="message-body" />);

    fireEvent.click(screen.getByText("Hidden Person"));
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(document.querySelector("[data-spoiler]")?.hasAttribute("data-revealed")).toBe(true);

    fireEvent.click(screen.getByText("Maya Okafor"));

    const card = await screen.findByRole("dialog");

    expect(within(card).getByRole("heading", { name: "Maya Okafor" })).toBeTruthy();
  });

  it("opens from a mention chip with Enter or Space, and Esc gives focus back to it", async () => {
    mutations.mergeUsers([userFixture(7, "Maya Okafor")]);

    const html =
      '<p>Ship it, <span class="mention" data-user-id="7"><span class="profile-card-name">' +
      "Maya Okafor</span></span></p>";

    await renderWithCard(<BodyHtml html={html} className="message-body" />);

    const chip = screen.getByRole("button", { name: "Maya Okafor" });

    expect(chip.tagName).toBe("SPAN");
    expect(chip.tabIndex).toBe(0);
    expect(chip.getAttribute("aria-haspopup")).toBe("dialog");

    for (const key of ["Enter", " "]) {
      chip.focus();
      fireEvent.keyDown(chip, { key });

      const card = await screen.findByRole("dialog");

      expect(within(card).getByRole("heading", { name: "Maya Okafor" })).toBeTruthy();
      fireEvent.keyDown(document.activeElement ?? card, { key: "Escape" });
      await expectClosed();
      expect(document.activeElement).toBe(chip);
    }
  });

  it("keeps a mention a covered spoiler hides out of the tab order until it is revealed", async () => {
    mutations.mergeUsers([userFixture(9, "Hidden Person")]);

    const html =
      '<p>And <span class="spoiler" data-spoiler=""><span class="mention" data-user-id="9">' +
      '<span class="profile-card-name">Hidden Person</span></span></span></p>';

    await renderWithCard(<BodyHtml html={html} className="message-body" />);

    const chip = document.querySelector<HTMLElement>('.mention[data-user-id="9"]');

    expect(chip?.hasAttribute("role")).toBe(false);
    expect(chip?.tabIndex).toBe(-1);
    expect(screen.queryByRole("button", { name: "Hidden Person" })).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Spoiler, activate to reveal" }));

    const revealed = screen.getByRole("button", { name: "Hidden Person" });

    expect(revealed).toBe(chip);
    expect(revealed.tabIndex).toBe(0);
    fireEvent.keyDown(revealed, { key: "Enter" });
    expect(await screen.findByRole("dialog")).toBeTruthy();
  });
});
