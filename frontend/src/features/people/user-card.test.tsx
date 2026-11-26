import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { type ReactNode, useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  meFixture,
  messageFixture,
  pageFixture,
  roomDetailFixture,
  sidebarRowFixture,
  userFixture,
} from "../../api/testing.ts";
import type { ActivityItem } from "../../gen/ActivityItem.ts";
import type { PersonProfile } from "../../gen/PersonProfile.ts";
import type { User } from "../../gen/User.ts";
import type { UserRole } from "../../gen/UserRole.ts";
import { beginRoomRequest } from "../../store/join-state.ts";
import { initialState } from "../../store/state.ts";
import { mutations, store } from "../../store/store.ts";
import { peoplePages } from "../../sync/admin.ts";
import { directs } from "../../sync/directs.ts";
import { ActivityRow } from "../activity/activity-row.tsx";
import { BodyHtml } from "../messages/body-html.tsx";
import { ReplyQuote } from "../messages/message-content.tsx";
import { MemberRow } from "../panes/members-pane.tsx";
import { RoomIntro } from "../room/dividers.tsx";
import { MessageRow } from "../room/message-row.tsx";
import { SavedRow } from "../saved/saved-row.tsx";
import { HitRow } from "../search/hit-row.tsx";
import { threadFixture } from "../threads/test-fixtures.ts";
import { ThreadRow } from "../threads/threads-pane.tsx";
import { cachedUserCardProfile, clearUserCardProfiles, UserCardHost } from "./user-card.tsx";
import { CLOSE_DELAY_MS, OPEN_DELAY_MS, resetUserCard } from "./user-card-store.ts";
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

  vi.spyOn(peoplePages, "cardProfile").mockImplementation((userId) => {
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
    vi.mocked(peoplePages.cardProfile).mockResolvedValue(
      profileOf(user, { emailAddress: "ashley@example.com" }),
    );
    await renderWithCard(<UserCardTrigger userId={PERSON}>Ashley</UserCardTrigger>);

    const card = await openCard("Ashley");

    expect(within(card).getByText("Admin")).toBeTruthy();

    const email = await within(card).findByRole("link", { name: "ashley@example.com" });

    expect(email.getAttribute("href")).toBe("mailto:ashley@example.com");
    expect(peoplePages.cardProfile).toHaveBeenCalledWith(PERSON, expect.any(AbortSignal));
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
    vi.mocked(peoplePages.cardProfile).mockResolvedValue(
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
    vi.mocked(peoplePages.cardProfile).mockResolvedValue(
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

describe("what the card fetches and keeps", () => {
  it("hides the email at once when a sync demotes the viewer, and stops serving the admin's copy", async () => {
    const user = person();
    const viewer = { ...userFixture(VIEWER, "Viewer"), role: "administrator" as const };

    signInAs("administrator");
    mutations.mergeUsers([user, viewer]);
    vi.mocked(peoplePages.cardProfile).mockImplementation(() =>
      Promise.resolve(
        profileOf(user, {
          emailAddress:
            store.getState().users[VIEWER]?.role === "administrator" ? "ashley@example.com" : null,
        }),
      ),
    );
    await renderWithCard(<UserCardTrigger userId={PERSON}>Ashley</UserCardTrigger>);

    const card = await openCard("Ashley");

    await within(card).findByRole("link", { name: "ashley@example.com" });

    // The demotion lands in `users` by revision, as sync delivers it; `me` still says administrator.
    act(() =>
      mutations.mergeUsers([
        { ...viewer, role: "member", updatedAt: "2026-02-01T00:00:00.000000Z" },
      ]),
    );
    expect(store.getState().me?.user.role).toBe("administrator");
    expect(within(card).queryByRole("link", { name: "ashley@example.com" })).toBeNull();

    // The administrator's copy no longer answers for the viewer: a member's read replaces it.
    await waitFor(() => expect(peoplePages.cardProfile).toHaveBeenCalledTimes(2));
    await waitFor(() => expect(cachedUserCardProfile(PERSON)?.emailAddress).toBeNull());
    fireEvent.keyDown(document.body, { key: "Escape" });
    await expectClosed();

    const again = await openCard("Ashley");

    expect(peoplePages.cardProfile).toHaveBeenCalledTimes(2);
    expect(within(again).queryByRole("link", { name: "ashley@example.com" })).toBeNull();
  });

  it("reads the card's profile, keeping only what it shows and never a sign-in link", async () => {
    const user = person();

    signInAs("administrator");
    mutations.mergeUsers([user]);
    vi.mocked(peoplePages.cardProfile).mockResolvedValue(
      profileOf(user, {
        emailAddress: "ashley@example.com",
        transferUrl: "https://chat.example/session/transfers/t",
        transferQrSvg: "<svg/>",
        canBan: true,
      }),
    );
    await renderWithCard(<UserCardTrigger userId={PERSON}>Ashley</UserCardTrigger>);

    const card = await openCard("Ashley");

    await within(card).findByRole("link", { name: "ashley@example.com" });
    expect(peoplePages.cardProfile).toHaveBeenCalledWith(PERSON, expect.any(AbortSignal));
    expect(Object.keys(cachedUserCardProfile(PERSON) ?? {}).sort()).toEqual([
      "emailAddress",
      "ignoredByMe",
      "status",
      "user",
    ]);
  });

  it("forgets a cached profile once it is stale, and fetches it again", async () => {
    mutations.mergeUsers([person()]);
    await renderWithCard(<UserCardTrigger userId={PERSON}>Ashley</UserCardTrigger>);

    await openCard("Ashley");
    await waitFor(() => expect(cachedUserCardProfile(PERSON)).toBeDefined());
    fireEvent.keyDown(document.body, { key: "Escape" });
    await expectClosed();

    const later = Date.now() + 60_000;

    vi.spyOn(Date, "now").mockReturnValue(later);
    vi.mocked(peoplePages.cardProfile).mockReturnValue(new Promise(() => undefined));
    await openCard("Ashley");

    expect(cachedUserCardProfile(PERSON)).toBeUndefined();
    expect(peoplePages.cardProfile).toHaveBeenCalledTimes(2);
  });
});

describe("Message on a card", () => {
  /** A direct-message create the test lands when it chooses. */
  function heldCreate(): (row: ReturnType<typeof sidebarRowFixture>) => void {
    let land: (row: ReturnType<typeof sidebarRowFixture>) => void = () => undefined;

    vi.spyOn(directs, "create").mockReturnValue(
      new Promise((resolve) => {
        land = resolve;
      }),
    );

    return (row) => land(row);
  }

  it("still opens the conversation when the pointer leaves a hover-opened card after Message", async () => {
    mutations.mergeUsers([person()]);

    const land = heldCreate();

    const pathname = await renderWithCard(
      <UserCardTrigger userId={PERSON}>Ashley</UserCardTrigger>,
    );

    const trigger = screen.getByRole("button", { name: "Ashley" });

    fireEvent.pointerEnter(trigger, { pointerType: "mouse" });

    const card = await screen.findByRole("dialog", {}, { timeout: OPEN_DELAY_MS * 3 });

    fireEvent.pointerLeave(trigger, { pointerType: "mouse" });
    fireEvent.pointerEnter(card, { pointerType: "mouse" });
    fireEvent.click(within(card).getByRole("button", { name: "Message" }));
    fireEvent.pointerLeave(card, { pointerType: "mouse" });
    await act(() => new Promise((resolve) => setTimeout(resolve, CLOSE_DELAY_MS + 100)));
    expect(screen.getByRole("dialog")).toBe(card);

    await act(async () => land(sidebarRowFixture(55, "Ashley", "direct", [PERSON])));
    await waitFor(() => expect(pathname()).toBe("/r/55"));
    await expectClosed();
  });

  it("stays put when the same person's card is dismissed and opened again before it lands", async () => {
    mutations.mergeUsers([person()]);

    const land = heldCreate();

    const pathname = await renderWithCard(
      <UserCardTrigger userId={PERSON}>Ashley</UserCardTrigger>,
    );

    const card = await openCard("Ashley");

    fireEvent.click(within(card).getByRole("button", { name: "Message" }));
    fireEvent.keyDown(document.body, { key: "Escape" });
    await expectClosed();

    const fresh = await openCard("Ashley");

    expect(within(fresh).getByRole("button", { name: "Message" }).hasAttribute("aria-busy")).toBe(
      false,
    );
    await act(async () => land(sidebarRowFixture(55, "Ashley", "direct", [PERSON])));
    expect(pathname()).toBe("/");
    expect(screen.getByRole("dialog")).toBe(fresh);
  });
});

describe("a card off a mention whose body changes", () => {
  it("closes when the body's markup is replaced in place", async () => {
    mutations.mergeUsers([userFixture(7, "Maya Okafor")]);

    const mention =
      '<span class="mention" data-user-id="7"><span class="profile-card-name">Maya Okafor</span></span>';

    let edit: (html: string) => void = () => undefined;

    function Editable() {
      const [html, setHtml] = useState(`<p>Ship it, ${mention}</p>`);

      edit = setHtml;

      return <BodyHtml html={html} className="message-body" />;
    }

    await renderWithCard(<Editable />);
    fireEvent.click(screen.getByText("Maya Okafor"));
    await screen.findByRole("dialog");

    act(() => edit(`<p>Ship it today, ${mention}</p>`));
    await expectClosed();
  });
});

/** A mention of Maya Okafor (7) in message HTML, as the server writes it. */
const MAYA_MENTION =
  '<p>Ask <span class="mention" data-user-id="7"><span class="profile-card-name">Maya Okafor' +
  "</span></span> first</p>";

const LATER = Date.parse("2026-10-06T16:00:00.000Z");

/** Expects the card for `name` to be up, then dismisses it. */
async function expectCardFor(name: string): Promise<void> {
  const card = await screen.findByRole("dialog");

  expect(within(card).getByRole("heading", { name })).toBeTruthy();
  fireEvent.keyDown(document.body, { key: "Escape" });
  await expectClosed();
}

/**
 * The author's name button for `name` opens their card on a click and from the keyboard, and the
 * avatar beside it on a click; `onOpen` (the row's own action) never runs for any of them.
 */
async function expectNameOpensCard(
  name: string,
  onOpen: ReturnType<typeof vi.fn> | null,
): Promise<void> {
  const user = userEvent.setup();
  const button = screen.getByRole("button", { name });

  expect(button.dataset.userCardTrigger).toBeDefined();
  expect(button.tabIndex).toBe(0);
  expect(button.getAttribute("aria-hidden")).toBeNull();

  await user.click(button);
  await expectCardFor(name);

  for (const key of ["{Enter}", " "]) {
    button.focus();
    await user.keyboard(key);
    await expectCardFor(name);
  }

  const avatar = document.querySelector<HTMLElement>(
    `[data-user-card-trigger="${button.dataset.userCardTrigger}"][aria-hidden="true"]`,
  );

  if (avatar !== null) {
    await user.click(avatar);
    await expectCardFor(name);
  }

  if (onOpen !== null) {
    expect(onOpen).not.toHaveBeenCalled();
  }
}

/** The mention chip for Maya opens her card on a click and from the keyboard, marking itself open. */
async function expectMentionOpensCard(onOpen: ReturnType<typeof vi.fn> | null): Promise<void> {
  const user = userEvent.setup();
  const chip = screen.getByRole("button", { name: "Maya Okafor" });

  expect(chip.tagName).toBe("SPAN");
  expect(chip.getAttribute("aria-expanded")).toBe("false");

  await user.click(screen.getByText("Maya Okafor"));
  await screen.findByRole("dialog");
  expect(chip.getAttribute("aria-expanded")).toBe("true");
  await expectCardFor("Maya Okafor");
  expect(chip.getAttribute("aria-expanded")).toBe("false");

  chip.focus();
  fireEvent.keyDown(chip, { key: "Enter" });
  await expectCardFor("Maya Okafor");

  if (onOpen !== null) {
    expect(onOpen).not.toHaveBeenCalled();
  }
}

describe("cards from lists, results and quotes", () => {
  beforeEach(() => {
    mutations.mergeUsers([userFixture(7, "Maya Okafor"), userFixture(8, "Lucía Fernández")]);
  });

  it("opens from a saved item's author and its mentions, and the row still opens the message", async () => {
    const handlers = {
      onOpen: vi.fn(),
      onToggleDone: vi.fn(),
      onRemind: vi.fn(),
      onCustomRemind: vi.fn(),
      onRemove: vi.fn(),
      onMenu: vi.fn(),
    };

    await renderWithCard(
      <SavedRow
        item={{
          id: 1,
          messageId: 3,
          status: "in_progress",
          remindAt: null,
          remindedAt: null,
          createdAt: "2026-10-06T09:00:00.000Z",
        }}
        message={messageFixture(3, 1, { creatorId: 8, bodyHtml: MAYA_MENTION })}
        conversation={null}
        now={LATER}
        motion={undefined}
        celebrate={false}
        handlers={handlers}
      />,
    );

    await expectNameOpensCard("Lucía Fernández", handlers.onOpen);
    await expectMentionOpensCard(handlers.onOpen);

    fireEvent.click(screen.getByRole("button", { name: /^Open Lucía Fernández's message/ }));
    expect(handlers.onOpen).toHaveBeenCalledTimes(1);
  });

  it("opens from a search result's author and its mentions, its link still jumping to the message", async () => {
    const pathname = await renderWithCard(
      <HitRow
        hit={messageFixture(41, 1, { creatorId: 8, bodyHtml: MAYA_MENTION })}
        conversation={undefined}
        terms={[]}
        now={LATER}
      />,
    );

    await expectNameOpensCard("Lucía Fernández", null);
    expect(pathname()).toBe("/");
    await expectMentionOpensCard(null);
    expect(pathname()).toBe("/");

    const link = document.querySelector<HTMLElement>("[data-search-hit-link]");

    expect(link?.getAttribute("aria-label")).toMatch(/^Lucía Fernández, /);
    expect(link?.textContent).toBe("");
    fireEvent.click(link ?? document.body);
    await waitFor(() => expect(pathname()).toBe("/r/1/m/41"));
  });

  it("opens your own card from your member row, and someone else's without messaging them", async () => {
    const onOpen = vi.fn();

    const member = {
      userId: 8,
      name: "Lucía Fernández",
      presence: "online",
      statusText: null,
      starred: false,
      bot: false,
      viewer: false,
    } as const;

    await renderWithCard(
      <ul>
        <MemberRow member={member} onOpen={onOpen} onStar={vi.fn()} />
        <MemberRow
          member={{ ...member, userId: 7, name: "Maya Okafor", viewer: true }}
          onOpen={onOpen}
          onStar={vi.fn()}
        />
      </ul>,
    );

    await expectNameOpensCard("Lucía Fernández", onOpen);
    await expectNameOpensCard("Maya Okafor", onOpen);
    expect(screen.queryByRole("button", { name: "Message Maya Okafor" })).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Message Lucía Fernández" }));
    expect(onOpen).toHaveBeenCalledWith(member);
  });

  it("opens from an activity row's creator, and the row still opens the item", async () => {
    const onOpen = vi.fn();

    const item: ActivityItem = {
      id: 7,
      eventType: "mention",
      state: "unread",
      readAt: null,
      handledAt: null,
      createdAt: "2026-10-06T11:00:00.000Z",
      updatedAt: "2026-10-06T11:00:00.000Z",
      source: {
        sourceType: "message",
        sourceId: 9001,
        roomId: 12,
        threadId: null,
        messageId: 9001,
        eventId: null,
        creatorId: 8,
        title: "general",
        body: "Can you review the launch plan?",
        occurredAt: "2026-10-06T11:00:00.000Z",
        approvalStatus: null,
        budgetCap: null,
        path: "/rooms/12/@9001",
      },
    };

    await renderWithCard(
      <ActivityRow
        item={item}
        now={LATER}
        motion={undefined}
        celebrate={false}
        onOpen={onOpen}
        onAction={vi.fn()}
        onMenu={vi.fn()}
      />,
    );

    await expectNameOpensCard("Lucía Fernández", onOpen);

    fireEvent.click(screen.getByRole("button", { name: /^Unread, Mention, general/ }));
    expect(onOpen).toHaveBeenCalledWith(item);
  });

  it("opens from a thread row's starter, and the row still opens the thread", async () => {
    const onOpen = vi.fn();

    await renderWithCard(
      <ul>
        <ThreadRow thread={threadFixture(5, { creatorId: 8, name: "Launch" })} onOpen={onOpen} />
      </ul>,
    );

    await expectNameOpensCard("Lucía Fernández", onOpen);

    fireEvent.click(screen.getByRole("button", { name: /^Launch, Lucía Fernández, 1 reply/ }));
    expect(onOpen).toHaveBeenCalledTimes(1);
  });

  it("opens from a reply quote's author, and the quote still jumps to the original", async () => {
    const source = messageFixture(21, 1, { creatorId: 8 });

    mutations.applyPage(1, pageFixture([source]), "replace");

    const pathname = await renderWithCard(
      <ReplyQuote message={messageFixture(22, 1, { replyToMessageId: source.id })} />,
    );

    await expectNameOpensCard("Lucía Fernández", null);
    expect(pathname()).toBe("/");

    const link = document.querySelector<HTMLElement>(".message-reply-link");

    expect(link?.getAttribute("href")).toBe("/app/r/1/m/21");
    expect(link?.querySelector("[data-user-card-trigger]")).toBeNull();
  });

  it("raises an agent author's link above a reply quote and hovers its card", async () => {
    const agent = {
      ...userFixture(12, "Deploy Agent"),
      role: "bot" as const,
      agent: { agentId: 3, kind: "workspace" as const, status: "idle" as const, suspended: false },
    };

    const source = messageFixture(25, 1, { creatorId: 12 });

    mutations.mergeUsers([agent]);
    mutations.applyPage(1, pageFixture([source]), "replace");

    const pathname = await renderWithCard(
      <ReplyQuote message={messageFixture(26, 1, { replyToMessageId: source.id })} />,
    );

    const link = document.querySelector<HTMLElement>(
      ".message-reply > .user-card-hover > .agent-link",
    );

    expect(link).not.toBeNull();
    fireEvent.pointerEnter(link ?? document.body, { pointerType: "mouse" });
    expect(await screen.findByRole("dialog", {}, { timeout: OPEN_DELAY_MS * 3 })).toBeTruthy();

    fireEvent.click(link ?? document.body);
    await waitFor(() => expect(pathname()).toBe("/agents/3"));
  });

  it("opens your own card from a note-to-self intro", async () => {
    const viewer = userFixture(VIEWER, "Samira Haddad");
    const row = sidebarRowFixture(19, viewer.name, "direct", [VIEWER]);

    signInAs("member");
    mutations.mergeUsers([viewer]);
    mutations.setRoomDetail(
      {
        ...roomDetailFixture(19),
        room: row.room,
        membership: row.membership,
        displayName: viewer.name,
        sidebarDisplayName: viewer.name,
        directMemberIds: [VIEWER],
        users: [viewer],
        noteToSelf: true,
      },
      beginRoomRequest(),
    );

    await renderWithCard(<RoomIntro roomId={19} />);

    const user = userEvent.setup();
    const button = screen.getByRole("button", { name: /Samira Haddad/ });

    expect(button.dataset.userCardTrigger).toBe(String(VIEWER));
    await user.click(button);
    await expectCardFor("Samira Haddad(you)");

    for (const key of ["{Enter}", " "]) {
      button.focus();
      await user.keyboard(key);
      await expectCardFor("Samira Haddad(you)");
    }

    const avatar = document.querySelector<HTMLElement>(
      `[data-user-card-trigger="${VIEWER}"][aria-hidden="true"]`,
    );

    expect(avatar).not.toBeNull();
    await user.click(avatar ?? document.body);
    await expectCardFor("Samira Haddad(you)");
  });

  it("does not cache an aborted admin profile reply after a viewer demotion", async () => {
    const user = person();

    let landAdmin: (reply: PersonProfile) => void = () => undefined;

    signInAs("administrator");
    mutations.mergeUsers([user]);
    vi.mocked(peoplePages.cardProfile)
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            landAdmin = resolve;
          }),
      )
      .mockResolvedValueOnce(profileOf(user, { emailAddress: null }));

    await renderWithCard(<UserCardTrigger userId={PERSON}>Ashley</UserCardTrigger>);

    const card = await openCard("Ashley");

    act(() =>
      mutations.mergeUsers([
        {
          ...userFixture(VIEWER, "Samira Haddad"),
          role: "member",
          updatedAt: "2026-02-01T00:00:00.000000Z",
        },
      ]),
    );
    await waitFor(() => expect(cachedUserCardProfile(PERSON)).toBeDefined());
    await act(async () => landAdmin(profileOf(user, { emailAddress: "ashley@example.com" })));
    expect(cachedUserCardProfile(PERSON)?.emailAddress).toBeNull();
    expect(within(card).queryByRole("link", { name: "ashley@example.com" })).toBeNull();
  });
});
