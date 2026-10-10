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
import { afterAll, beforeAll, beforeEach, describe, expect, it } from "vitest";
import { forbidden } from "../../../mock/http.ts";
import { CARD_IDS } from "../../../mock/s3/cards.ts";
import { USER_IDS } from "../../../mock/seed.ts";
import type { GithubDiscussion } from "../../gen/GithubDiscussion.ts";
import type { Me } from "../../gen/Me.ts";
import type { MessageCard } from "../../gen/MessageCard.ts";
import type { MessagePage } from "../../gen/MessagePage.ts";
import type { PollResults } from "../../gen/PollResults.ts";
import { githubKey } from "../../store/cards.ts";
import type { MessageDTO, SyncEvent } from "../../store/model.ts";
import { mutations, store, useStore } from "../../store/store.ts";
import { installMockNetwork, type MockNetwork } from "../../test/mock-network.ts";
import { toastSnapshot } from "../../ui/toast-store.ts";
import { loadCustomIcons } from "../messages/commands.ts";
import { CreatePollDialog, filledOptions, pollProblems } from "./create-poll-dialog.tsx";
import {
  githubDraftCount,
  patchGithubDraft,
  readGithubDraft,
  resetGithubDrafts,
} from "./github-drafts.ts";
import MessageCards from "./message-cards.tsx";
import { PollCard } from "./poll-card.tsx";

// The cards render against the in-memory mock backend (mock/s3/cards.ts) through stubbed fetch:
// the per-viewer previews, votes and responses go through the real actions.
let network: MockNetwork;

const ROOM = CARD_IDS.room;

const { events, messages, polls } = CARD_IDS;

function headers() {
  return { "Content-Type": "application/json", "X-CSRF-Token": network.server.csrfToken() };
}

/** A message of the cards room as the store holds it (the page is loaded before each test). */
function held(id: number): MessageDTO {
  const message = store.getState().messages[id];

  if (message === undefined) {
    throw new Error(`message ${id} isn't in the store`);
  }

  return message;
}

/** Renders `children` inside a small router with the routes the cards link to. */
async function inRouter(children: () => ReactNode) {
  const rootRoute = createRootRoute({ component: () => <>{children()}</> });
  const roomRoute = createRoute({ getParentRoute: () => rootRoute, path: "/r/$roomId" });

  const permalinkRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/r/$roomId/m/$messageId",
  });

  const newThreadRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/r/$roomId/t/new",
    validateSearch: (search: { readonly parent?: number }) => ({
      parent: Number(search.parent ?? 0),
    }),
  });

  const threadRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/r/$roomId/t/$threadId",
    validateSearch: (search: { readonly m?: number }) =>
      search.m === undefined ? {} : { m: Number(search.m) },
  });

  const router = createRouter({
    routeTree: rootRoute.addChildren([roomRoute, permalinkRoute, newThreadRoute, threadRoute]),
    history: createMemoryHistory({ initialEntries: [`/r/${ROOM}`] }),
  });

  const view = render(<RouterProvider router={router} />);

  await act(() => router.load());

  return { router, view };
}

/** The cards under one message of the cards room, following the store as it changes. */
function Live({ id, threadId = null }: { readonly id: number; readonly threadId?: number | null }) {
  return <MessageCards message={held(id)} threadId={threadId} />;
}

function LiveRow({ id, threadId }: { readonly id: number; readonly threadId?: number | null }) {
  // Subscribing here re-renders the cards when the message changes in the store.
  const message = useStore((state) => state.messages[id]);

  return message === undefined ? null : <Live id={id} threadId={threadId ?? null} />;
}

async function renderCards(id: number, threadId: number | null = null) {
  return inRouter(() => <LiveRow id={id} threadId={threadId} />);
}

/** The text a comment or note field is holding. */
function fieldValue(element: HTMLElement): string {
  if (element instanceof HTMLTextAreaElement || element instanceof HTMLInputElement) {
    return element.value;
  }

  throw new Error("expected a text field");
}

/** The poll's vote count as read aloud (the animated digits are hidden from it). */
function total(poll: HTMLElement): string {
  return `${poll.querySelector(".poll-total .visually-hidden")?.textContent ?? ""} votes`;
}

/**
 * Makes the mock refuse the requests `matches` picks (403, `message`) until the returned function
 * is called. Patches the server the stubbed fetch already talks to.
 */
function refuse(matches: (method: string, path: string) => boolean, message: string): () => void {
  const { server } = network;
  const handle = server.handle;

  server.handle = (request) =>
    matches(request.method, request.path)
      ? Promise.resolve({ status: 403, json: { error: forbidden(message).error } })
      : handle(request);

  return () => {
    server.handle = handle;
  };
}

/** Makes the requests `matches` picks wait until the returned function lets them through. */
function hold(matches: (method: string, path: string) => boolean): () => void {
  const { server } = network;
  const handle = server.handle;
  const waiting: (() => void)[] = [];

  server.handle = (request) =>
    matches(request.method, request.path)
      ? new Promise<void>((resolve) => waiting.push(resolve)).then(() => handle(request))
      : handle(request);

  return () => {
    server.handle = handle;

    for (const release of waiting) {
      release();
    }
  };
}

async function control(body: Readonly<Record<string, number | string | readonly number[]>>) {
  await fetch("/__mock/cards", { method: "POST", headers: headers(), body: JSON.stringify(body) });
}

const meta = document.createElement("meta");

beforeAll(() => {
  // One network for the file: the actions' runtime keeps the fetch it first saw.
  network = installMockNetwork();
  meta.name = "csrf-token";
  document.head.append(meta);

  // jsdom doesn't lay out; the router restores scroll on navigation.
  window.scrollTo = () => undefined;
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

  // A freshly seeded backend for each test, since votes and responses change it.
  network.server.reset();
  meta.content = network.server.csrfToken();

  const me: Me = await (await fetch("/api/v1/me")).json();
  const page: MessagePage = await (await fetch(`/api/v1/rooms/${ROOM}/messages`)).json();

  mutations.reset();
  resetGithubDrafts();
  mutations.setMe(me);
  mutations.applyPage(ROOM, page, "replace");
});

/** Classic Discuss: the mapping row, which is what makes write actions available. */
async function discuss(parentMessageId: number): Promise<number> {
  const response = await fetch(
    `/api/v1/rooms/${ROOM}/github/pull_requests/${CARD_IDS.pullRequests.open}/discussion`,
    {
      method: "POST",
      headers: headers(),
      body: JSON.stringify({ messageId: parentMessageId }),
    },
  );

  const created: GithubDiscussion = await response.json();

  return created.threadId;
}

/** A checks-style `message.cards`: the preview stays up and is fetched again. */
function refreshPreview(id: number): void {
  const message = held(id);

  const event: SyncEvent = {
    type: "message.cards",
    seq: 1,
    topic: `room:${ROOM}`,
    data: {
      messageId: id,
      roomId: ROOM,
      threadId: message.threadId,
      cards: message.cards,
      asOf: new Date(Date.parse(message.cardsAsOf) + 1000).toISOString(),
    },
  };

  mutations.applyEvents([event], Date.now());
}

describe("polls", () => {
  it("lets an administrator end another person's poll and shows final results", async () => {
    const user = userEvent.setup();
    await renderCards(messages.pollOpen);
    const card = screen.getByRole("region", { name: "Poll" });
    await user.click(within(card).getByRole("button", { name: "End poll now" }));
    await waitFor(() => expect(within(card).getByText("Closed")).toBeTruthy());
    expect(within(card).getByRole("contentinfo").textContent).toContain("Final results");
    expect(within(card).queryByRole("radio")).toBeNull();
    expect(within(card).queryByRole("button", { name: "End poll now" })).toBeNull();
    expect(total(card)).toBe("4 votes");

    const results: PollResults = await (
      await fetch(`/api/v1/rooms/${ROOM}/polls/${polls.open}`)
    ).json();

    expect(results.poll.closedAt).not.toBeNull();
    expect(results.myOptionIds).toEqual([]);
  });

  it("lets a member end their own poll", async () => {
    const me = store.getState().me;

    if (me === null) throw new Error("expected viewer");
    mutations.setMe({ ...me, user: { ...me.user, role: "member" } });

    const response = await fetch(`/api/v1/rooms/${ROOM}/polls`, {
      method: "POST",
      headers: headers(),
      body: JSON.stringify({
        clientMessageId: "own-poll",
        question: "Lunch?",
        options: ["Pizza", "Tacos"],
        multiple: false,
        anonymous: false,
        closesAt: null,
      }),
    });

    const posted: MessageDTO = await response.json();
    mutations.receiveMessage(posted);
    await renderCards(posted.id);
    await userEvent.setup().click(screen.getByRole("button", { name: "End poll now" }));
    await screen.findByText("Closed");
    expect(screen.getByRole("contentinfo").textContent).toContain("Final results");
  });

  it("hides End poll now from other members and on closed polls", async () => {
    const me = store.getState().me;

    if (me === null) throw new Error("expected viewer");
    expect(held(messages.pollOpen).creatorId).not.toBe(me.user.id);
    mutations.setMe({ ...me, user: { ...me.user, role: "member" } });
    const rendered = await renderCards(messages.pollOpen);
    expect(screen.queryByRole("button", { name: "End poll now" })).toBeNull();
    rendered.view.unmount();
    mutations.setMe(me);
    await renderCards(messages.pollClosed);
    expect(screen.queryByRole("button", { name: "End poll now" })).toBeNull();
  });

  it("keeps voting open and reports a refused end request", async () => {
    const allow = refuse(
      (method, path) => method === "POST" && path.endsWith("/end"),
      "Permission changed",
    );

    try {
      await renderCards(messages.pollOpen);
      await userEvent.setup().click(screen.getByRole("button", { name: "End poll now" }));
      await waitFor(() =>
        expect(screen.getByRole("status").textContent).toBe(
          "Couldn't end this poll. Permission changed",
        ),
      );
      expect(screen.getByRole("radio", { name: "Tacos" })).toBeTruthy();
      expect(screen.getByRole("button", { name: "End poll now" }).hasAttribute("disabled")).toBe(
        false,
      );
    } finally {
      allow();
    }
  });

  it("votes from a radio group, then changes and takes the vote back", async () => {
    const user = userEvent.setup();

    await renderCards(messages.pollOpen);

    const poll = screen.getByRole("region", { name: "Poll" });
    const group = within(poll).getByRole("group", { name: "Cast your vote" });
    const vote = within(poll).getByRole("button", { name: "Vote" });
    const said = () => within(poll).getByRole("status").textContent;

    expect(total(poll)).toBe("4 votes");
    expect(vote.hasAttribute("disabled")).toBe(true);
    await user.click(within(group).getByRole("radio", { name: "Tacos" }));
    await user.click(vote);
    await waitFor(() => expect(total(poll)).toBe("5 votes"));
    expect(within(poll).getByText("(your vote)")).toBeTruthy();
    expect(poll.querySelector(".poll-result[data-mine]")?.textContent).toContain("60%");

    // Focus moves on to Change vote, which replaces the options; the result is said aloud.
    expect(document.activeElement).toBe(within(poll).getByRole("button", { name: "Change vote" }));
    await waitFor(() => expect(said()).toMatch(/^Voted for Tacos\. Results: Tacos 60%, Pizza/));

    await user.click(within(poll).getByRole("button", { name: "Change vote" }));

    const tacos = within(poll).getByRole<HTMLInputElement>("radio", { name: "Tacos" });

    expect(tacos.checked).toBe(true);
    expect(document.activeElement).toBe(tacos);

    // The arrow keys move the choice within the group, as radios do.
    await user.keyboard("{ArrowDown}");
    expect(within(poll).getByRole<HTMLInputElement>("radio", { name: "Pizza" }).checked).toBe(true);
    await user.click(within(poll).getByRole("button", { name: "Vote" }));
    await waitFor(() =>
      expect(poll.querySelector(".poll-result[data-mine] .poll-result-text")?.textContent).toBe(
        "Pizza",
      ),
    );

    await user.click(within(poll).getByRole("button", { name: "Retract" }));
    await waitFor(() => expect(within(poll).getByRole("radio", { name: "Tacos" })).toBeTruthy());
    expect(total(poll)).toBe("4 votes");
    expect(document.activeElement).toBe(within(poll).getByRole("radio", { name: "Tacos" }));
    await waitFor(() => expect(said()).toBe("Vote retracted."));
  });

  it("keeps a refused vote's options, says why, and keeps focus on them", async () => {
    const user = userEvent.setup();

    const allow = refuse(
      (method, path) => method === "POST" && path.endsWith(`/polls/${polls.open}/vote`),
      "This poll is closed",
    );

    await renderCards(messages.pollOpen);

    const poll = screen.getByRole("region", { name: "Poll" });

    await user.click(within(poll).getByRole("radio", { name: "Sushi" }));
    await user.click(within(poll).getByRole("button", { name: "Vote" }));

    const message = "Couldn't record your vote. This poll is closed";

    await waitFor(() => expect(within(poll).getByRole("status").textContent).toBe(message));
    expect(poll.querySelector(".card-error")?.textContent).toBe(message);
    expect(total(poll)).toBe("4 votes");
    expect(document.activeElement).toBe(within(poll).getByRole("radio", { name: "Tacos" }));
    allow();
  });

  it("collects ticks before voting in a multiple-choice poll", async () => {
    const user = userEvent.setup();

    await renderCards(messages.pollMultiple);

    const poll = screen.getByRole("region", { name: "Poll" });

    expect(within(poll).getByText(/Multiple choice/)).toBeTruthy();
    await user.click(within(poll).getByRole("button", { name: "Change vote" }));
    expect(within(poll).getByRole("group", { name: /^Cast your vote/ })).toBeTruthy();

    const vote = within(poll).getByRole("button", { name: "Vote" });

    await user.click(within(poll).getByLabelText("Slack import"));
    await user.click(within(poll).getByLabelText("Polls"));
    expect(vote.hasAttribute("disabled")).toBe(true);
    await user.click(within(poll).getByLabelText("Boards"));
    await user.click(vote);
    await waitFor(() => expect(within(poll).getByRole("button", { name: "Retract" })).toBeTruthy());
    expect(poll.querySelector(".poll-result[data-mine] .poll-result-text")?.textContent).toBe(
      "Boards",
    );
  });

  it("shows a closed poll's final results with no way to vote", async () => {
    await renderCards(messages.pollClosed);

    const poll = screen.getByRole("region", { name: "Poll" });

    expect(within(poll).getByText("Closed")).toBeTruthy();
    expect(within(poll).getByText(/Final results/)).toBeTruthy();
    expect(within(poll).queryByRole("button")).toBeNull();
    expect(poll.querySelector(".poll-result[data-leading] .poll-result-text")?.textContent).toBe(
      "Wednesday",
    );
  });

  it("fetches the viewer's own vote in an anonymous poll and shows no voters", async () => {
    await renderCards(messages.pollAnonymous);

    const poll = screen.getByRole("region", { name: "Poll" });

    expect(within(poll).getByText(/Closes in 2 hours/)).toBeTruthy();
    await waitFor(() => expect(within(poll).getByText(/You voted/)).toBeTruthy());
    expect(poll.querySelector(".poll-result[data-mine] .poll-result-text")?.textContent).toBe(
      "Very",
    );
    expect(poll.querySelector(".poll-voters")).toBeNull();
  });

  it("offers Try again when the viewer's results couldn't be fetched, and loads them on it", async () => {
    const user = userEvent.setup();

    const allow = refuse(
      (method, path) => method === "GET" && path.endsWith(`/polls/${polls.anonymous}`),
      "Not now",
    );

    await renderCards(messages.pollAnonymous);

    const poll = screen.getByRole("region", { name: "Poll" });
    const again = await within(poll).findByRole("button", { name: "Try again" });

    expect(within(poll).getByText("Couldn't load this poll's results.")).toBeTruthy();
    allow();
    await user.click(again);
    await waitFor(() => expect(within(poll).getByText(/You voted/)).toBeTruthy());
    expect(within(poll).queryByRole("button", { name: "Try again" })).toBeNull();
  });

  it("follows another member's vote and the poll closing", async () => {
    await renderCards(messages.pollOpen);

    const poll = screen.getByRole("region", { name: "Poll" });

    // The mock publishes poll.updated; feed it to the store as the sync engine would.
    const publish = async (seq: number) => {
      const fetched: PollResults = await (
        await fetch(`/api/v1/rooms/${ROOM}/polls/${polls.open}`)
      ).json();

      act(() =>
        mutations.applyEvents(
          [
            {
              type: "poll.updated",
              seq,
              topic: `room:${ROOM}`,
              data: { roomId: ROOM, threadId: null, poll: fetched.poll },
            },
          ],
          Date.now(),
        ),
      );
    };

    await control({
      op: "vote",
      userId: USER_IDS.theo,
      pollId: polls.open,
      optionIds: [polls.open * 10 + 1],
    });
    await publish(1);
    await waitFor(() => expect(total(poll)).toBe("5 votes"));

    await control({ op: "close-poll", pollId: polls.open });
    await publish(2);
    await waitFor(() => expect(within(poll).getByText("Closed")).toBeTruthy());
    expect(within(poll).queryByRole("radio", { name: "Tacos" })).toBeNull();
  });
});

describe("events", () => {
  it("answers an event at once, with the counts, and offers every future occurrence", async () => {
    const user = userEvent.setup();

    await renderCards(messages.eventRecurring);

    const event = screen.getByRole("region", { name: "Event: Weekly product sync" });
    const going = await within(event).findByRole("button", { name: /^Going/ });

    expect(going.textContent).toContain("2");
    expect(within(event).getByText("Repeats")).toBeTruthy();
    expect(
      within(event).getByRole("link", { name: "Weekly product sync" }).getAttribute("href"),
    ).toBe(`/r/${ROOM}/events/${events.recurring}`);
    expect(
      within(event).getByRole("link", { name: "Join with Google Meet" }).getAttribute("href"),
    ).toBe("https://meet.google.com/abc-defg-hij");

    await user.click(within(event).getByLabelText("Apply to all future occurrences"));
    await user.click(going);
    expect(going.getAttribute("aria-pressed")).toBe("true");
    expect(going.textContent).toContain("3");
    await user.click(within(event).getByRole("button", { name: /^Can't go/ }));
    await waitFor(() => expect(going.textContent).toContain("2"));
  });

  it("shows an answer at once and marks it pending until the server has it", async () => {
    const user = userEvent.setup();

    await renderCards(messages.eventRecurring);

    const event = screen.getByRole("region", { name: "Event: Weekly product sync" });
    const going = await within(event).findByRole("button", { name: /^Going/ });
    const group = within(event).getByRole("group", { name: "Your response" });

    const release = hold(
      (method, path) => method === "PUT" && path.endsWith(`/events/${events.recurring}/attendance`),
    );

    await user.click(going);
    expect(going.getAttribute("aria-pressed")).toBe("true");
    expect(going.hasAttribute("data-pending")).toBe(true);
    expect(group.getAttribute("aria-busy")).toBe("true");

    // A second answer shows at once too, queued behind the first.
    const maybe = within(event).getByRole("button", { name: /^Maybe/ });

    await user.click(maybe);
    expect(maybe.getAttribute("aria-pressed")).toBe("true");
    expect(maybe.hasAttribute("data-pending")).toBe(true);

    release();
    await waitFor(() => expect(group.hasAttribute("aria-busy")).toBe(false));
    expect(maybe.getAttribute("aria-pressed")).toBe("true");
    expect(maybe.hasAttribute("data-pending")).toBe(false);
  });

  it("strikes a cancelled event through and takes no responses", async () => {
    await renderCards(messages.eventCancelled);

    const event = screen.getByRole("region", { name: "Event: Launch retro" });

    expect(within(event).getByText("Cancelled")).toBeTruthy();
    expect(within(event).getByRole("link", { name: /Lounge/ })).toBeTruthy();
    expect(within(event).queryByRole("button")).toBeNull();
  });
});

describe("GitHub pull requests", () => {
  it("shows the status, review, checks and Discuss once fetched", async () => {
    await renderCards(messages.githubOpen);

    const card = await screen.findByRole("region", {
      name: "Pull request: Rate limit the sync endpoint with a token bucket",
    });

    expect(within(card).getByText("Open")).toBeTruthy();
    expect(within(card).getByText("Approved")).toBeTruthy();
    expect(within(card).getByText("Checks passing")).toBeTruthy();
    expect(within(card).getByRole("button", { name: "Discuss" })).toBeTruthy();
  });

  it("shows a draft, and keeps a frame while GitHub is still fetching", async () => {
    await renderCards(messages.githubDraftAndLoading);

    await screen.findByText("Draft");
    expect(document.querySelectorAll('.github-card[aria-busy="true"]')).toHaveLength(1);
  });

  it("offers Retry when the fetch failed and shows nothing for a private repository", async () => {
    await renderCards(messages.githubFailedAndHidden);

    expect(await screen.findByText(/Couldn't load this pull request/)).toBeTruthy();
    expect(screen.getByRole("button", { name: "Retry" })).toBeTruthy();
    expect(document.querySelectorAll(".github-card")).toHaveLength(1);
  });

  it("lists the changed files when it heads the pull request's discussion thread", async () => {
    const threadId = await discuss(messages.githubOpen);

    await renderCards(messages.githubOpen, threadId);

    expect(await screen.findByText("4 files changed")).toBeTruthy();
    expect(screen.queryByRole("link", { name: "Discuss" })).toBeNull();
    expect(await screen.findByRole("button", { name: "Comment" })).toBeTruthy();
  });

  it("comments, requests changes and asks for reviewers from the card", async () => {
    const user = userEvent.setup();

    await discuss(messages.githubOpen);
    await renderCards(messages.githubOpen);

    const card = await screen.findByRole("region", {
      name: "Pull request: Rate limit the sync endpoint with a token bucket",
    });

    await user.click(within(card).getByRole("button", { name: "Comment" }));
    expect(within(card).getByRole("alert").textContent).toBe("Write a comment first.");

    await user.type(within(card).getByRole("textbox", { name: "Comment" }), "Looks good.");
    await user.click(within(card).getByRole("button", { name: "Comment" }));
    expect((await within(card).findByRole("status")).textContent).toBe(
      "Comment posted on GitHub as @maya.",
    );
    expect(githubDraftCount()).toBe(0);

    await user.click(within(card).getByRole("button", { name: "Review" }));
    await user.click(await screen.findByRole("menuitem", { name: "Request changes" }));

    const dialog = await screen.findByRole("dialog", { name: "Request changes" });

    await user.click(within(dialog).getByRole("button", { name: "Request changes" }));
    expect(within(dialog).getByRole("alert").textContent).toBe(
      "Add a note describing the requested changes.",
    );

    await user.type(within(dialog).getByRole("textbox", { name: "Note" }), "Rename the limiter.");
    await user.click(within(dialog).getByRole("button", { name: "Request changes" }));
    await waitFor(() => expect(within(card).getByText("Changes requested")).toBeTruthy());

    await user.click(within(card).getByRole("button", { name: "Request reviewers" }));

    const reviewers = await screen.findByRole("dialog", { name: "Request reviewers" });

    await user.type(within(reviewers).getByRole("textbox", { name: "Reviewers" }), "alice, @bob");
    await user.click(within(reviewers).getByRole("button", { name: "Request reviewers" }));
    await waitFor(() =>
      expect(
        toastSnapshot().some(
          (item) => item.title === "Requested review from @alice, @bob on GitHub as @maya.",
        ),
      ).toBe(true),
    );
  });

  it("hides comment, review and reviewer request when the account can't post", async () => {
    await discuss(messages.githubOpen);
    await control({ op: "github-writes", roomId: ROOM, enabled: 0 });
    await renderCards(messages.githubOpen);

    const card = await screen.findByRole("region", {
      name: "Pull request: Rate limit the sync endpoint with a token bucket",
    });

    await waitFor(async () => {
      const response = await fetch("/__mock/cards", {
        method: "POST",
        headers: headers(),
        body: JSON.stringify({ op: "fetches", roomId: ROOM }),
      });

      const fetches: Record<string, number> = await response.json();

      expect(fetches[`github-actions:${CARD_IDS.pullRequests.open}`]).toBeGreaterThan(0);
    });
    expect(within(card).queryByRole("button", { name: "Comment" })).toBeNull();
    expect(within(card).queryByRole("button", { name: "Review" })).toBeNull();
    expect(within(card).queryByRole("button", { name: "Request reviewers" })).toBeNull();
  });

  const OPEN = "Pull request: Rate limit the sync endpoint with a token bucket";

  it("keeps a typed comment and an open review dialog across a preview refresh", async () => {
    const user = userEvent.setup();

    await discuss(messages.githubOpen);
    await renderCards(messages.githubOpen);

    const card = await screen.findByRole("region", { name: OPEN });

    await user.type(within(card).getByRole("textbox", { name: "Comment" }), "Keep this.");
    await user.click(within(card).getByRole("button", { name: "Review" }));
    await user.click(await screen.findByRole("menuitem", { name: "Request changes" }));

    const dialog = await screen.findByRole("dialog", { name: "Request changes" });
    const note = within(dialog).getByRole("textbox", { name: "Note" });

    await user.type(note, "Don't lose me.");

    const release = hold((method, path) => method === "GET" && path.includes("/card"));

    act(() => {
      refreshPreview(messages.githubOpen);
    });

    await waitFor(() => {
      const preview =
        store.getState().cards.previews.github[
          githubKey(ROOM, CARD_IDS.pullRequests.open, { messageId: messages.githubOpen })
        ];

      expect(preview?.status).toBe("loading");
    });

    expect(document.querySelector('.github-card[aria-busy="true"]')).toBeNull();
    expect(fieldValue(within(card).getByRole("textbox", { name: "Comment" }))).toBe("Keep this.");
    expect(screen.getByRole("dialog", { name: "Request changes" })).toBeTruthy();
    expect(fieldValue(within(dialog).getByRole("textbox", { name: "Note" }))).toBe(
      "Don't lose me.",
    );
    expect(document.activeElement).toBe(note);

    release();
  });

  it("does not post a review again when its dialog is reopened while the write is pending", async () => {
    const user = userEvent.setup();

    await discuss(messages.githubOpen);
    await renderCards(messages.githubOpen);

    const card = await screen.findByRole("region", { name: OPEN });
    let posts = 0;

    const release = hold((method, path) => {
      const matched = method === "POST" && path.endsWith("/reviews");

      if (matched) {
        posts += 1;
      }

      return matched;
    });

    await user.click(within(card).getByRole("button", { name: "Review" }));
    await user.click(await screen.findByRole("menuitem", { name: "Request changes" }));

    const dialog = await screen.findByRole("dialog", { name: "Request changes" });

    await user.type(within(dialog).getByRole("textbox", { name: "Note" }), "Rename it.");
    await user.click(within(dialog).getByRole("button", { name: "Request changes" }));
    await waitFor(() => expect(posts).toBe(1));
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("dialog")).toBeNull();

    // A pointer click in the menu's light-dismiss window is ignored; the keyboard opens it.
    within(card).getByRole("button", { name: "Review" }).focus();
    await user.keyboard("{ArrowDown}");
    await user.click(await screen.findByRole("menuitem", { name: "Request changes" }));

    const again = await screen.findByRole("dialog", { name: "Request changes" });

    expect(fieldValue(within(again).getByRole("textbox", { name: "Note" }))).toBe("Rename it.");
    await user.click(within(again).getByRole("button", { name: "Request changes" }));
    expect(posts).toBe(1);

    release();
    await waitFor(() =>
      expect(
        toastSnapshot().some((item) => item.title === "Requested changes on GitHub as @maya."),
      ).toBe(true),
    );
    expect(githubDraftCount()).toBe(0);
  });

  it("keeps the comment draft and retries after the server refuses", async () => {
    const user = userEvent.setup();

    await discuss(messages.githubOpen);
    await renderCards(messages.githubOpen);

    const card = await screen.findByRole("region", { name: OPEN });

    await user.type(within(card).getByRole("textbox", { name: "Comment" }), "Looks good.");

    const restore = refuse(
      (method, path) => method === "POST" && path.endsWith("/comments"),
      "GitHub refused: no",
    );

    await user.click(within(card).getByRole("button", { name: "Comment" }));
    expect((await within(card).findByRole("alert")).textContent).toBe("GitHub refused: no");
    expect(fieldValue(within(card).getByRole("textbox", { name: "Comment" }))).toBe("Looks good.");
    expect(toastSnapshot().some((item) => item.title === "GitHub refused: no")).toBe(true);

    restore();
    await user.click(within(card).getByRole("button", { name: "Comment" }));
    expect((await within(card).findByRole("status")).textContent).toBe(
      "Comment posted on GitHub as @maya.",
    );
  });

  it("keeps drafts apart by room and drops one that is empty and idle", () => {
    const pullRequestId = CARD_IDS.pullRequests.open;

    patchGithubDraft(ROOM, pullRequestId, { comment: "only here" });
    expect(readGithubDraft(ROOM + 1, pullRequestId).comment).toBe("");
    expect(readGithubDraft(ROOM, pullRequestId).comment).toBe("only here");
    expect(githubDraftCount()).toBe(1);

    patchGithubDraft(ROOM, pullRequestId, { comment: "", commentPending: true });
    expect(githubDraftCount()).toBe(1);
    expect(readGithubDraft(ROOM, pullRequestId).commentPending).toBe(true);

    patchGithubDraft(ROOM, pullRequestId, { commentPending: false });
    expect(githubDraftCount()).toBe(0);
    expect(readGithubDraft(ROOM, pullRequestId).comment).toBe("");
  });

  it("shows write actions after Discuss on a mounted card", async () => {
    const user = userEvent.setup();

    await renderCards(messages.githubOpen);

    const card = await screen.findByRole("region", { name: OPEN });

    expect(within(card).queryByRole("button", { name: "Comment" })).toBeNull();
    await user.click(within(card).getByRole("button", { name: "Discuss" }));
    expect(await within(card).findByRole("button", { name: "Comment" })).toBeTruthy();
  });

  it("opens one dialog when the pull request is mounted twice", async () => {
    const user = userEvent.setup();
    const threadId = await discuss(messages.githubOpen);

    function Both() {
      return (
        <>
          <LiveRow id={messages.githubOpen} />
          <LiveRow id={messages.githubOpen} threadId={threadId} />
        </>
      );
    }

    await inRouter(() => <Both />);

    const cards = await screen.findAllByRole("region", { name: OPEN });

    expect(cards).toHaveLength(2);

    const timeline = cards[0];

    if (timeline === undefined) {
      throw new Error("expected the timeline card");
    }

    await user.click(within(timeline).getByRole("button", { name: "Review" }));
    await user.click(await screen.findByRole("menuitem", { name: "Approve" }));
    expect(screen.getAllByRole("dialog")).toHaveLength(1);

    await user.keyboard("{Escape}");
    expect(screen.queryByRole("dialog")).toBeNull();

    await user.click(within(timeline).getByRole("button", { name: "Request reviewers" }));
    expect(screen.getAllByRole("dialog")).toHaveLength(1);
  });

  it("closes a dialog when the copy that opened it unmounts, and keeps the note", async () => {
    const user = userEvent.setup();
    const threadId = await discuss(messages.githubOpen);

    function Pair() {
      const [timeline, setTimeline] = useState(true);

      return (
        <>
          <button type="button" onClick={() => setTimeline(false)}>
            Hide timeline
          </button>
          {timeline ? <LiveRow id={messages.githubOpen} /> : null}
          <LiveRow id={messages.githubOpen} threadId={threadId} />
        </>
      );
    }

    await inRouter(() => <Pair />);

    const cards = await screen.findAllByRole("region", { name: OPEN });
    const timeline = cards[0];

    if (timeline === undefined) {
      throw new Error("expected the timeline card");
    }

    await user.click(within(timeline).getByRole("button", { name: "Review" }));
    await user.click(await screen.findByRole("menuitem", { name: "Request changes" }));

    const dialog = await screen.findByRole("dialog", { name: "Request changes" });

    await user.type(within(dialog).getByRole("textbox", { name: "Note" }), "Keep the note.");
    await user.click(screen.getByRole("button", { name: "Hide timeline" }));

    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.getAllByRole("region", { name: OPEN })).toHaveLength(1);

    const draft = readGithubDraft(ROOM, CARD_IDS.pullRequests.open);

    expect(draft.review).toBeNull();
    expect(draft.reviewNote).toBe("Keep the note.");
  });

  it("closes the reviewers dialog when the copy that opened it unmounts", async () => {
    const user = userEvent.setup();
    const threadId = await discuss(messages.githubOpen);

    function Pair() {
      const [timeline, setTimeline] = useState(true);

      return (
        <>
          <button type="button" onClick={() => setTimeline(false)}>
            Hide timeline
          </button>
          {timeline ? <LiveRow id={messages.githubOpen} /> : null}
          <LiveRow id={messages.githubOpen} threadId={threadId} />
        </>
      );
    }

    await inRouter(() => <Pair />);

    const cards = await screen.findAllByRole("region", { name: OPEN });
    const timeline = cards[0];

    if (timeline === undefined) {
      throw new Error("expected the timeline card");
    }

    await user.click(within(timeline).getByRole("button", { name: "Request reviewers" }));

    const dialog = await screen.findByRole("dialog", { name: "Request reviewers" });

    await user.type(within(dialog).getByRole("textbox", { name: "Reviewers" }), "alice");
    await user.click(screen.getByRole("button", { name: "Hide timeline" }));

    expect(screen.queryByRole("dialog")).toBeNull();

    const draft = readGithubDraft(ROOM, CARD_IDS.pullRequests.open);

    expect(draft.reviewersOpen).toBe(false);
    expect(draft.reviewersOwner).toBeNull();
    expect(draft.reviewers).toBe("alice");
  });

  it("closes the reviewers dialog when the card's key changes", async () => {
    const user = userEvent.setup();

    await discuss(messages.githubOpen);

    function Keyed() {
      const [generation, setGeneration] = useState(0);

      return (
        <>
          <button type="button" onClick={() => setGeneration((count) => count + 1)}>
            Remount
          </button>
          <LiveRow key={generation} id={messages.githubOpen} />
        </>
      );
    }

    await inRouter(() => <Keyed />);

    const card = await screen.findByRole("region", { name: OPEN });

    await user.click(within(card).getByRole("button", { name: "Request reviewers" }));

    const dialog = await screen.findByRole("dialog", { name: "Request reviewers" });

    await user.type(within(dialog).getByRole("textbox", { name: "Reviewers" }), "bob");
    await user.click(screen.getByRole("button", { name: "Remount" }));

    expect(screen.queryByRole("dialog")).toBeNull();

    const draft = readGithubDraft(ROOM, CARD_IDS.pullRequests.open);

    expect(draft.reviewersOwner).toBeNull();
    expect(draft.reviewers).toBe("bob");
  });

  async function actionFetches(): Promise<number> {
    const response = await fetch("/__mock/cards", {
      method: "POST",
      headers: headers(),
      body: JSON.stringify({ op: "fetches", roomId: ROOM }),
    });

    const fetches: Record<string, number> = await response.json();

    return fetches[`github-actions:${CARD_IDS.pullRequests.open}`] ?? 0;
  }

  it("asks for actions once when two copies finish loading", async () => {
    const threadId = await discuss(messages.githubOpen);

    function Both() {
      return (
        <>
          <LiveRow id={messages.githubOpen} />
          <LiveRow id={messages.githubOpen} threadId={threadId} />
        </>
      );
    }

    await inRouter(() => <Both />);
    expect(await screen.findAllByRole("button", { name: "Comment" })).toHaveLength(2);
    expect(await actionFetches()).toBe(1);
  });

  async function cardFetches(): Promise<number> {
    const response = await fetch("/__mock/cards", {
      method: "POST",
      headers: headers(),
      body: JSON.stringify({ op: "fetches", roomId: ROOM }),
    });

    const fetches: Record<string, number> = await response.json();

    return fetches[`github:${CARD_IDS.pullRequests.open}`] ?? 0;
  }

  it("asks for actions once when a refresh finishes", async () => {
    await discuss(messages.githubOpen);
    await renderCards(messages.githubOpen);
    await screen.findByRole("button", { name: "Comment" });

    const before = await actionFetches();
    const cardsBefore = await cardFetches();

    act(() => {
      refreshPreview(messages.githubOpen);
    });

    await waitFor(async () => {
      expect(await cardFetches()).toBeGreaterThan(cardsBefore);
      expect(await actionFetches()).toBe(before + 1);
    });
    expect(await actionFetches()).toBe(before + 1);
  });

  it("keeps the pull request and offers Retry when a refresh fails", async () => {
    const user = userEvent.setup();

    await renderCards(messages.githubOpen);

    const card = await screen.findByRole("region", { name: OPEN });

    const restore = refuse(
      (method, path) => method === "GET" && path.includes("/card"),
      "GitHub is down",
    );

    act(() => {
      refreshPreview(messages.githubOpen);
    });

    expect((await within(card).findByRole("alert")).textContent).toBe(
      "Couldn't refresh this pull request. GitHub is down",
    );
    expect(within(card).getByText("Approved")).toBeTruthy();
    expect(within(card).getByRole("button", { name: "Retry" })).toBeTruthy();

    restore();
    await user.click(within(card).getByRole("button", { name: "Retry" }));
    await waitFor(() => expect(within(card).queryByRole("alert")).toBeNull());
    expect(within(card).getByText("Approved")).toBeTruthy();
  });

  it("refetches a failed refresh when the preview is invalidated again", async () => {
    await renderCards(messages.githubOpen);

    const card = await screen.findByRole("region", { name: OPEN });

    const restore = refuse(
      (method, path) => method === "GET" && path.includes("/card"),
      "GitHub is down",
    );

    act(() => {
      refreshPreview(messages.githubOpen);
    });

    expect(await within(card).findByRole("alert")).toBeTruthy();

    const before = await cardFetches();

    restore();

    act(() => {
      refreshPreview(messages.githubOpen);
    });

    await waitFor(async () => {
      expect(await cardFetches()).toBeGreaterThan(before);
    });
    await waitFor(() => expect(within(card).queryByRole("alert")).toBeNull());
    expect(within(card).getByText("Approved")).toBeTruthy();
  });
});

describe("posts, Fizzy cards, quotes and links", () => {
  it("shows an X post with its media, quote and counts", async () => {
    await renderCards(messages.xPost);

    const post = screen.getByRole("region", { name: /^Post by / });

    expect(post.querySelectorAll(".x-media img").length).toBeGreaterThan(0);
    expect(post.querySelector(".x-quote")).not.toBeNull();
    expect(within(post).getByText(/412 likes/)).toBeTruthy();
  });

  it("frames a post still loading and falls back to the link when it failed", async () => {
    await renderCards(messages.xLoadingAndFailed);

    expect(document.querySelector('.x-card[aria-busy="true"]')).not.toBeNull();
    expect(document.querySelector(".x-chip")?.getAttribute("href")).toContain("x.com/gone");
  });

  it("shows a Fizzy card as the viewer's account sees it", async () => {
    await renderCards(messages.fizzyLoaded);

    const card = await screen.findByRole("region", { name: /^Fizzy card: / });

    expect(within(card).getByText("In progress")).toBeTruthy();
    expect(within(card).getByText("#importer")).toBeTruthy();
  });

  it("says when Fizzy isn't connected, the card isn't there, or the fetch failed", async () => {
    await renderCards(messages.fizzyNotConnectedAndNotFound);

    expect(await screen.findByText(/Connect Fizzy/)).toBeTruthy();
    expect(screen.getByText(/This card isn't there/)).toBeTruthy();
  });

  it("offers Retry on a failed Fizzy fetch", async () => {
    await renderCards(messages.fizzyFailedAndLoading);

    expect(await screen.findByRole("button", { name: "Retry" })).toBeTruthy();
  });

  it("links a quote to its source message, inline or fetched", async () => {
    await renderCards(messages.quoteInline);

    const inline = screen.getByRole("link", { name: /^Quoted message from / });

    expect(inline.getAttribute("href")).toMatch(new RegExp(`^/r/${ROOM}/m/\\d+$`));
  });

  it("fetches a quote from another room, and shows nothing for one the viewer can't see", async () => {
    await renderCards(messages.quoteFetched);

    const fetched = await screen.findByRole("link", { name: /^Quoted message from / });

    expect(fetched.getAttribute("href")).toMatch(/^\/r\/\d+\/m\/\d+$/);
    await waitFor(() => expect(document.querySelectorAll(".quote-card")).toHaveLength(1));
  });

  it("loads LinkedIn's player only on request, and shows the chip without a title", async () => {
    const user = userEvent.setup();

    await renderCards(messages.linkedin);
    expect(document.querySelector("iframe")).toBeNull();
    await user.click(screen.getByRole("button", { name: "Show embedded post" }));
    expect(document.querySelector("iframe.linkedin-embed")).not.toBeNull();
  });

  it("shows the LinkedIn chip, link previews and the Drive file", async () => {
    await renderCards(messages.linkedinChip);
    expect(screen.getByRole("link", { name: /View post on LinkedIn/ })).toBeTruthy();
  });

  it("shows a page preview with its site and title", async () => {
    await renderCards(messages.linkImage);
    expect(document.querySelector(".link-card .link-title")?.textContent).toBe(
      "Fast by default: budgets that hold",
    );
  });

  it("shows a Drive file in the attachment slot", async () => {
    await renderCards(messages.drive);
    expect(screen.getByRole("link", { name: /Google Drive file/ }).getAttribute("href")).toContain(
      "drive.google.com",
    );
  });
});

describe("what renders nothing", () => {
  it("skips a kind this app doesn't know", async () => {
    await renderCards(messages.unknownKind);
    expect(document.querySelector(".message-cards")).toBeNull();
  });

  it("leaves out link previews the author hid, keeping the pull request", async () => {
    await renderCards(messages.suppressed);
    expect(await screen.findByText("Open")).toBeTruthy();
    expect(document.querySelector(".link-card")).toBeNull();
  });

  it("skips a known card it can't read, and still shows the others", async () => {
    // SAFETY: the wire keeps a card it can't read as it came; this one lacks its data.
    const broken = JSON.parse('{"kind":"x"}') as MessageCard;
    const message = held(messages.drive);

    mutations.receiveMessage({ ...message, cards: [broken, ...message.cards] });
    await renderCards(messages.drive);

    expect(screen.getByRole("link", { name: /Google Drive file/ })).toBeTruthy();
    expect(document.querySelector(".x-card")).toBeNull();
  });
});

describe("creating a poll", () => {
  it("checks the draft as the server will", () => {
    expect(pollProblems("", ["A"])).toEqual({
      question: "Ask a question.",
      options: "Give at least two options.",
    });
    expect(pollProblems("Lunch?", ["Tacos", " ", "Pizza"])).toEqual({
      question: undefined,
      options: undefined,
    });
    expect(pollProblems("Lunch?", ["x".repeat(201), "ok"]).options).toMatch(/200 characters/);
    expect(filledOptions([" Tacos ", "", "Pizza"])).toEqual(["Tacos", "Pizza"]);
  });

  it("posts the question with its options, once, and closes", async () => {
    const user = userEvent.setup();
    const changes: boolean[] = [];

    render(
      <CreatePollDialog
        roomId={ROOM}
        open
        onOpenChange={(open) => changes.push(open)}
        initialQuestion="Where should the offsite be?"
      />,
    );

    const dialog = screen.getByRole("dialog", { name: "Create a poll" });

    await user.click(within(dialog).getByRole("button", { name: "Post poll" }));
    expect(within(dialog).getByText("Give at least two options.")).toBeTruthy();

    await user.type(within(dialog).getByRole("textbox", { name: "Option 1" }), "Columbus");
    await user.type(within(dialog).getByRole("textbox", { name: "Option 2" }), "Cleveland");
    await user.click(within(dialog).getByRole("button", { name: "Add option" }));
    await user.type(within(dialog).getByRole("textbox", { name: "Option 3" }), "Dayton");
    await user.click(within(dialog).getByRole("button", { name: "Remove option 3" }));
    await user.click(within(dialog).getByRole("switch", { name: /Anonymous/ }));
    await user.click(within(dialog).getByRole("button", { name: "In 1 week" }));
    await user.click(within(dialog).getByRole("button", { name: "Post poll" }));

    await waitFor(() => expect(changes).toEqual([false]));

    const created = Object.values(store.getState().messages).filter(
      (message) => message.poll?.options.some((option) => option.label === "Columbus") ?? false,
    );

    expect(created).toHaveLength(1);
    expect(created[0]?.poll?.anonymous).toBe(true);
    expect(created[0]?.poll?.options.map((option) => option.label)).toEqual([
      "Columbus",
      "Cleveland",
    ]);
    expect(created[0]?.poll?.closesAt).not.toBeNull();
  });

  it("retries an unchanged draft under the same client id, and an edited one under a new id", async () => {
    const user = userEvent.setup();
    const ids: string[] = [];
    const { server } = network;
    const handle = server.handle;

    server.handle = (request) => {
      if (request.method !== "POST" || !request.path.endsWith(`/rooms/${ROOM}/polls`)) {
        return handle(request);
      }

      ids.push(/"clientMessageId":"([^"]+)"/.exec(JSON.stringify(request.body))?.[1] ?? "");

      return ids.length < 3
        ? Promise.resolve({ status: 403, json: { error: forbidden("Try later").error } })
        : handle(request);
    };

    try {
      render(
        <CreatePollDialog
          roomId={ROOM}
          open
          onOpenChange={() => undefined}
          initialQuestion="Pizza or tacos?"
        />,
      );

      const dialog = screen.getByRole("dialog", { name: "Create a poll" });
      const post = within(dialog).getByRole("button", { name: "Post poll" });

      await user.type(within(dialog).getByRole("textbox", { name: "Option 1" }), "Pizza");
      await user.type(within(dialog).getByRole("textbox", { name: "Option 2" }), "Tacos");
      await user.click(post);
      await within(dialog).findByText("Try later");
      await user.click(post);
      await waitFor(() => expect(ids).toHaveLength(2));
      await user.type(within(dialog).getByRole("textbox", { name: "Option 2" }), " al pastor");
      await user.click(post);
      await waitFor(() => expect(ids).toHaveLength(3));

      expect(ids[1]).toBe(ids[0]);
      expect(ids[2]).not.toBe(ids[0]);
    } finally {
      server.handle = handle;
    }
  });
});

it("uploads one image per poll option and offers the shared emoji picker", async () => {
  const user = userEvent.setup();
  render(<CreatePollDialog roomId={ROOM} open onOpenChange={() => undefined} />);
  const dialog = screen.getByRole("dialog", { name: "Create a poll" });
  expect(within(dialog).getByRole("button", { name: "Emoji for option 1" })).toBeTruthy();
  expect(within(dialog).getByLabelText("Image for option 2")).toBeTruthy();

  const png = Uint8Array.from(
    atob(
      "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQIHWP4z8DwHwAFgAI/ScLbtAAAAABJRU5ErkJggg==",
    ),
    (c) => c.charCodeAt(0),
  );

  await user.upload(
    within(dialog).getByLabelText("Image for option 2"),
    new File([png], "pizza.png", { type: "image/png" }),
  );
  await user.type(within(dialog).getByRole("textbox", { name: "Question" }), "Lunch media?");
  await user.type(within(dialog).getByRole("textbox", { name: "Option 1" }), "Taco");
  await user.type(within(dialog).getByRole("textbox", { name: "Option 2" }), "Pizza");
  await user.click(within(dialog).getByRole("button", { name: "Post poll" }));
  await waitFor(() =>
    expect(
      Object.values(store.getState().messages).find((m) => m.markdownSource === "Lunch media?")
        ?.poll?.options[1]?.media,
    ).toMatchObject({ kind: "image" }),
  );
  expect(
    Object.values(store.getState().messages).find((m) => m.markdownSource === "Lunch media?")?.poll
      ?.options[1]?.media,
  ).toMatchObject({ kind: "image", url: expect.stringContaining("/rails/active_storage/") });
});

it("shows option images and inline emoji in choices and final results, using stills under reduced motion", async () => {
  const message = held(messages.pollOpen);
  const original = message.poll;

  if (original === null) throw new Error("expected seeded poll");

  const poll = {
    ...original,
    options: original.options.map((option, index) => ({
      ...option,
      media:
        index === 0
          ? ({ kind: "image", url: "/poll.gif", stillUrl: "/poll.png" } as const)
          : ({ kind: "emoji", content: "🌮" } as const),
    })),
  };

  document.documentElement.dataset.motion = "reduce";
  const view = render(<PollCard message={message} poll={{ ...poll, closed: true }} />);
  expect(screen.getByRole("img", { name: poll.options[0]?.label ?? "" }).getAttribute("src")).toBe(
    "/poll.png",
  );
  expect(screen.getAllByText("🌮")).toHaveLength(poll.options.length - 1);
  view.rerender(
    <PollCard
      message={message}
      poll={{ ...poll, options: poll.options.map((o) => ({ ...o, voterIds: [] })) }}
    />,
  );
  expect(screen.getByRole("img", { name: poll.options[0]?.label ?? "" }).getAttribute("src")).toBe(
    "/poll.png",
  );
  expect(screen.getAllByText("🌮")).toHaveLength(poll.options.length - 1);
  delete document.documentElement.dataset.motion;
});

it("shows the custom emoji name when its image fails to load", () => {
  const message = held(messages.pollOpen);
  const original = message.poll;

  if (original === null) throw new Error("expected seeded poll");

  const poll = {
    ...original,
    closed: true,
    options: original.options.map((option, index) =>
      index === 0 ? { ...option, media: { kind: "emoji", content: ":party:" } as const } : option,
    ),
  };

  const view = render(<PollCard message={message} poll={poll} />);
  const image = view.container.querySelector<HTMLImageElement>(".poll-option-emoji img");

  if (image === null) throw new Error("expected a custom emoji image");

  fireEvent.error(image);
  expect(screen.getByText(":party:")).toBeTruthy();
  expect(view.container.querySelector(".poll-option-emoji img")).toBeNull();
});

it("gives custom poll emoji their shortcode as alternative text", () => {
  const message = held(messages.pollOpen);
  const original = message.poll;

  if (original === null) throw new Error("expected seeded poll");

  const view = render(
    <PollCard
      message={message}
      poll={{
        ...original,
        closed: true,
        options: original.options.map((option, index) =>
          index === 0 ? { ...option, media: { kind: "emoji", content: ":party:" } } : option,
        ),
      }}
    />,
  );

  expect(view.container.querySelector(".poll-option-emoji img")?.getAttribute("alt")).toBe(
    ":party:",
  );
});

it("keeps a loaded custom emoji image when it is absent from the cached catalog", async () => {
  const icons = await loadCustomIcons();
  expect(icons.some((icon) => icon.content === ":deleted_party:")).toBe(false);
  const message = held(messages.pollOpen);
  const original = message.poll;

  if (original === null) throw new Error("expected seeded poll");

  const view = render(
    <PollCard
      message={message}
      poll={{
        ...original,
        closed: true,
        options: original.options.map((option, index) =>
          index === 0
            ? { ...option, media: { kind: "emoji", content: ":deleted_party:" } }
            : option,
        ),
      }}
    />,
  );

  const image = view.container.querySelector<HTMLImageElement>(".poll-option-emoji img");

  if (image === null) throw new Error("expected a custom emoji image");

  await act(async () => {
    fireEvent.load(image);
  });
  expect(view.container.querySelector(".poll-option-emoji img")).toBe(image);
  expect(image.getAttribute("src")).toBe("/icons/deleted_party");
  expect(screen.queryByText(":deleted_party:")).toBeNull();
});
