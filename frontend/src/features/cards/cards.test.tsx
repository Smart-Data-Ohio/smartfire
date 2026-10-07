import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterAll, beforeAll, beforeEach, describe, expect, it } from "vitest";
import { CARD_IDS } from "../../../mock/s3/cards.ts";
import { USER_IDS } from "../../../mock/seed.ts";
import type { Me } from "../../gen/Me.ts";
import type { MessageCard } from "../../gen/MessageCard.ts";
import type { MessagePage } from "../../gen/MessagePage.ts";
import type { PollResults } from "../../gen/PollResults.ts";
import type { ThreadCreated } from "../../gen/ThreadCreated.ts";
import type { MessageDTO } from "../../store/model.ts";
import { mutations, store, useStore } from "../../store/store.ts";
import { installMockNetwork, type MockNetwork } from "../../test/mock-network.ts";
import { CreatePollDialog, filledOptions, pollProblems } from "./create-poll-dialog.tsx";
import MessageCards from "./message-cards.tsx";

// The cards render against the in-memory mock backend (mock/s3/cards.ts) through stubbed fetch:
// the per-viewer previews, votes and responses go through the real actions.
let network: MockNetwork;

const ROOM = CARD_IDS.room;

const { messages, polls } = CARD_IDS;

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

/** The poll's vote count as read aloud (the animated digits are hidden from it). */
function total(poll: HTMLElement): string {
  return `${poll.querySelector(".poll-total .visually-hidden")?.textContent ?? ""} votes`;
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
  mutations.setMe(me);
  mutations.applyPage(ROOM, page, "replace");
});

describe("polls", () => {
  it("votes with one click, then changes and takes the vote back", async () => {
    const user = userEvent.setup();

    await renderCards(messages.pollOpen);

    const poll = screen.getByRole("region", { name: "Poll" });

    expect(total(poll)).toBe("4 votes");
    await user.click(within(poll).getByRole("button", { name: "Tacos" }));
    await waitFor(() => expect(total(poll)).toBe("5 votes"));
    expect(within(poll).getByText("(your vote)")).toBeTruthy();
    expect(poll.querySelector(".poll-result[data-mine]")?.textContent).toContain("60%");

    await user.click(within(poll).getByRole("button", { name: "Change vote" }));
    expect(within(poll).getByRole("button", { name: "Tacos" }).getAttribute("aria-pressed")).toBe(
      "true",
    );
    await user.click(within(poll).getByRole("button", { name: "Pizza" }));
    await waitFor(() =>
      expect(poll.querySelector(".poll-result[data-mine] .poll-result-text")?.textContent).toBe(
        "Pizza",
      ),
    );

    await user.click(within(poll).getByRole("button", { name: "Retract" }));
    await waitFor(() => expect(within(poll).getByRole("button", { name: "Tacos" })).toBeTruthy());
    expect(total(poll)).toBe("4 votes");
  });

  it("collects ticks before voting in a multiple-choice poll", async () => {
    const user = userEvent.setup();

    await renderCards(messages.pollMultiple);

    const poll = screen.getByRole("region", { name: "Poll" });

    expect(within(poll).getByText(/Multiple choice/)).toBeTruthy();
    await user.click(within(poll).getByRole("button", { name: "Change vote" }));

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
    expect(within(poll).queryByRole("button", { name: "Tacos" })).toBeNull();
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
      within(event).getByRole("link", { name: "Join with Google Meet" }).getAttribute("href"),
    ).toBe("https://meet.google.com/abc-defg-hij");

    await user.click(within(event).getByLabelText("Apply to all future occurrences"));
    await user.click(going);
    expect(going.getAttribute("aria-pressed")).toBe("true");
    expect(going.textContent).toContain("3");
    await user.click(within(event).getByRole("button", { name: /^Can't go/ }));
    await waitFor(() => expect(going.textContent).toContain("2"));
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
    expect(within(card).getByRole("link", { name: "Discuss" }).getAttribute("href")).toBe(
      `/r/${ROOM}/t/new?parent=${messages.githubOpen}`,
    );
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
    const response = await fetch(`/api/v1/rooms/${ROOM}/threads`, {
      method: "POST",
      headers: headers(),
      body: JSON.stringify({
        parentMessageId: messages.githubOpen,
        name: null,
        message: {
          clientMessageId: "cards-thread-1",
          markdownSource: "On it",
          replyToMessageId: null,
          replyNotifyAuthor: null,
        },
      }),
    });

    const created: ThreadCreated = await response.json();

    await renderCards(messages.githubOpen, created.message.threadId);

    expect(await screen.findByText("4 files changed")).toBeTruthy();
    expect(screen.queryByRole("link", { name: "Discuss" })).toBeNull();
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
});
