import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { type ReactNode, useState } from "react";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { meFixture, sidebarFixture, sidebarRowFixture } from "../../api/testing.ts";
import type { SearchChip } from "../../gen/SearchChip.ts";
import { loadSidebar, setMe } from "../../store/reducers.ts";
import { emptyResultList, initialSearchState, searchStore } from "../../store/search.ts";
import type { SearchResultsView } from "../../store/search-hooks.ts";
import { initialState } from "../../store/state.ts";
import { store } from "../../store/store.ts";
import { messageFixture } from "../threads/test-fixtures.ts";
import { HeaderSearch } from "./header-search.tsx";
import { SearchBox } from "./search-box.tsx";
import { SearchResults } from "./search-results.tsx";

/** `ui` in a small router with the routes search links to, at `/search`. */
async function renderRouted(ui: () => ReactNode) {
  const rootRoute = createRootRoute({ component: ui });
  const searchRoute = createRoute({ getParentRoute: () => rootRoute, path: "/search" });
  const roomRoute = createRoute({ getParentRoute: () => rootRoute, path: "/r/$roomId" });

  const messageRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/r/$roomId/m/$messageId",
  });

  const threadRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/r/$roomId/t/$threadId",
  });

  const router = createRouter({
    routeTree: rootRoute.addChildren([searchRoute, roomRoute, messageRoute, threadRoute]),
    history: createMemoryHistory({ initialEntries: ["/search"] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());

  return router;
}

const NOW = Date.parse("2026-10-06T16:00:00.000Z");

function view(extra: Partial<SearchResultsView>): SearchResultsView {
  return {
    ...emptyResultList,
    status: "ready",
    query: "launch",
    hasMore: false,
    loadMore: vi.fn(),
    reload: vi.fn(),
    ...extra,
  };
}

const PIN_CHIP: SearchChip = {
  operator: "has",
  value: "pin",
  token: "has:pin",
  label: "has: pin",
  removeQuery: "zebracorn",
};

// jsdom lays nothing out, so it has no scrollIntoView; the field scrolls the active row into view.
beforeAll(() => {
  Element.prototype.scrollIntoView = vi.fn();
});

afterEach(() => {
  store.setState(initialState, true);
  searchStore.setState(initialSearchState, true);
});

describe("SearchResults", () => {
  it("says search failed and retries", async () => {
    const results = view({ status: "error", error: "The server didn't answer." });
    const user = userEvent.setup();

    await renderRouted(() => (
      <SearchResults results={results} words={["launch"]} now={NOW} onQuery={vi.fn()} />
    ));

    expect(screen.getByRole("heading", { name: "Search isn't working right now" })).toBeTruthy();
    expect(screen.getByText("The server didn't answer.")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Try again" }));
    expect(results.reload).toHaveBeenCalledOnce();
  });

  it("offers each filter's removal when nothing matches", async () => {
    const onQuery = vi.fn();
    const user = userEvent.setup();

    await renderRouted(() => (
      <SearchResults
        results={view({ query: "zebracorn has:pin", chips: [PIN_CHIP] })}
        words={["zebracorn"]}
        now={NOW}
        onQuery={onQuery}
      />
    ));

    expect(
      screen.getByRole("heading", { name: "No results for “zebracorn has:pin”" }),
    ).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "has: pin" }));
    expect(onQuery).toHaveBeenCalledWith("zebracorn");
  });

  it("suggests other words when nothing matches without filters", async () => {
    await renderRouted(() => (
      <SearchResults
        results={view({ query: "zebracorn" })}
        words={["zebracorn"]}
        now={NOW}
        onQuery={vi.fn()}
      />
    ));

    expect(screen.getByText(/Check the spelling/)).toBeTruthy();
  });

  it("groups hits by conversation, marks the words and links each to its message", async () => {
    const top = messageFixture(10, {
      roomId: 4,
      bodyHtml: "<p>The launch is Friday</p>",
      createdAt: "2026-10-06T15:00:00.000Z",
    });

    const reply = messageFixture(11, {
      roomId: 4,
      threadId: 9,
      bodyHtml: "<p>Launching early</p>",
      createdAt: "2026-10-06T14:00:00.000Z",
    });

    const results = view({
      messages: [top, reply],
      conversations: {
        "4:": {
          roomId: 4,
          threadId: null,
          roomKind: "open",
          roomName: "general",
          roomIconName: null,
          threadName: null,
        },
        "4:9": {
          roomId: 4,
          threadId: 9,
          roomKind: "open",
          roomName: "general",
          roomIconName: null,
          threadName: "Launch plan",
        },
      },
    });

    const router = await renderRouted(() => (
      <SearchResults results={results} words={["launch"]} now={NOW} onQuery={vi.fn()} />
    ));

    const messages = screen.getByRole("region", { name: "Messages" });
    const hits = messages.querySelectorAll("[data-search-hit]");

    expect(hits).toHaveLength(2);
    expect(within(messages).getByText("2")).toBeTruthy();
    expect(
      [...messages.querySelectorAll("mark.search-mark")].map((mark) => mark.textContent),
    ).toEqual(["launch", "Launching"]);

    expect(within(messages).getByRole("link", { name: /Launch plan/ })).toBeTruthy();

    const links = [...messages.querySelectorAll("[data-search-hit-link]")];

    expect(links.map((link) => link.getAttribute("href"))).toEqual(["/r/4/m/10", "/r/4/t/9?m=11"]);
    expect(screen.getByText("That's every message that matches.")).toBeTruthy();

    const user = userEvent.setup();

    await user.click(within(messages).getAllByRole("link", { name: /in general/ })[0] ?? messages);
    expect(router.state.location.pathname).toBe("/r/4/m/10");
  });

  it("hides a hit deleted since the search", async () => {
    store.setState({ tombstones: { 10: NOW } });

    await renderRouted(() => (
      <SearchResults
        results={view({ messages: [messageFixture(10)] })}
        words={["message"]}
        now={NOW}
        onQuery={vi.fn()}
      />
    ));

    expect(document.querySelectorAll("[data-search-hit]")).toHaveLength(0);
  });

  it("offers older results while there are more", async () => {
    const results = view({ messages: [messageFixture(10)], hasMore: true, nextCursor: "abc" });
    const user = userEvent.setup();

    await renderRouted(() => (
      <SearchResults results={results} words={[]} now={NOW} onQuery={vi.fn()} />
    ));

    expect(screen.getByText("1+")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Load more messages" }));
    expect(results.loadMore).toHaveBeenCalledOnce();
  });

  it("conceals a spoiler in a hit and reveals it without following the card", async () => {
    const user = userEvent.setup();

    const hit = messageFixture(12, {
      roomId: 4,
      bodyHtml: '<p>Friday <span class="spoiler" data-spoiler="">the launch is secret</span></p>',
      createdAt: "2026-10-06T15:00:00.000Z",
    });

    const results = view({
      messages: [hit],
      conversations: {
        "4:": {
          roomId: 4,
          threadId: null,
          roomKind: "open",
          roomName: "general",
          roomIconName: null,
          threadName: null,
        },
      },
    });

    await renderRouted(() => (
      <SearchResults results={results} words={["launch", "Friday"]} now={NOW} onQuery={vi.fn()} />
    ));

    const messages = screen.getByRole("region", { name: "Messages" });

    expect(messages.querySelector("mark.search-mark")?.textContent).toBe("Friday");
    expect(messages.querySelector(".spoiler mark")).toBeNull();

    const spoiler = within(messages).getByRole("button", { name: "Spoiler, activate to reveal" });

    expect(spoiler.textContent).toBe("the launch is secret");

    await user.click(spoiler);

    expect(spoiler.hasAttribute("data-revealed")).toBe(true);
    expect(spoiler.textContent).toBe("the launch is secret");
  });
});

/** A search field owning its value, as the page and header use it. */
function Field({
  onSearch,
  variant,
}: {
  onSearch: (query: string) => void;
  variant: "page" | "header";
}) {
  const [value, setValue] = useState("");

  return (
    <SearchBox
      variant={variant}
      value={value}
      onValueChange={setValue}
      onSearch={onSearch}
      placeholder="Search"
    />
  );
}

describe("SearchBox", () => {
  it("suggests recent searches when focused empty, and runs one", async () => {
    searchStore.setState({
      recents: {
        status: "ready",
        searches: [{ id: 1, query: "launch checklist", searchedAt: "2026-10-06T10:00:00.000Z" }],
      },
    });

    const onSearch = vi.fn();
    const user = userEvent.setup();

    await renderRouted(() => <Field variant="header" onSearch={onSearch} />);
    await user.click(screen.getByRole("combobox", { name: "Search messages" }));

    const recent = screen.getByRole("group", { name: "Recent searches" });

    await user.click(within(recent).getByRole("option", { name: /launch checklist/ }));
    expect(onSearch).toHaveBeenCalledWith("launch checklist");
  });

  it("finishes an operator from the keyboard, then submits", async () => {
    searchStore.setState({ recents: { status: "ready", searches: [] } });

    const onSearch = vi.fn();
    const user = userEvent.setup();

    await renderRouted(() => <Field variant="page" onSearch={onSearch} />);

    const input = screen.getByRole("combobox", { name: "Search messages" });

    await user.click(input);
    await user.keyboard("launch ha");
    expect(input.getAttribute("aria-expanded")).toBe("true");

    // "Search for …" first, then the operators the last word starts.
    await user.keyboard("{ArrowDown}{ArrowDown}");

    const active = document.getElementById(input.getAttribute("aria-activedescendant") ?? "");

    expect(active?.textContent).toContain("has:");
    await user.keyboard("{Enter}");
    // The operator stays open for its value: the suggestions turn to has: values.
    expect(input).toHaveProperty("value", "launch has:");

    await user.keyboard("file{Escape}");
    expect(input.getAttribute("aria-expanded")).toBe("false");
    await user.keyboard("{Enter}");
    expect(onSearch).toHaveBeenCalledWith("launch has:file");
  });

  it("points the active descendant at an option id with no spaces, even for a recent search", async () => {
    searchStore.setState({
      recents: {
        status: "ready",
        searches: [{ id: 1, query: "launch checklist", searchedAt: "2026-10-06T10:00:00.000Z" }],
      },
    });

    const user = userEvent.setup();

    await renderRouted(() => <Field variant="header" onSearch={vi.fn()} />);

    const input = screen.getByRole("combobox", { name: "Search messages" });

    await user.click(input);
    await user.keyboard("{ArrowDown}");

    const id = input.getAttribute("aria-activedescendant") ?? "";

    expect(id).not.toMatch(/\s/u);
    expect(document.getElementById(id)?.textContent).toContain("launch checklist");
  });

  it("leaves the page's empty field to the page's own lists", async () => {
    searchStore.setState({
      recents: {
        status: "ready",
        searches: [{ id: 1, query: "launch checklist", searchedAt: "2026-10-06T10:00:00.000Z" }],
      },
    });

    const user = userEvent.setup();

    await renderRouted(() => <Field variant="page" onSearch={vi.fn()} />);
    await user.click(screen.getByRole("combobox", { name: "Search messages" }));
    expect(screen.queryByRole("listbox")).toBeNull();
  });
});

describe("HeaderSearch", () => {
  const input = () => screen.getByRole<HTMLInputElement>("combobox", { name: "Search messages" });

  /** The viewer's rooms: #launch-planning (7), Launch Ops (8), a DM (9) and their notes (40). */
  function seedRooms() {
    const sidebar = sidebarFixture([
      sidebarRowFixture(7, "launch-planning", "closed"),
      sidebarRowFixture(8, "Launch Ops"),
      sidebarRowFixture(9, "Grace Hopper", "direct", [12]),
      { ...sidebarRowFixture(40, "Ada Lovelace", "direct", [meFixture.user.id]), noteToSelf: true },
    ]);

    sidebar.noteToSelfRoomId = 40;
    store.setState(loadSidebar(setMe(initialState, meFixture), sidebar, 0), true);
    searchStore.setState({ recents: { status: "ready", searches: [] } });
  }

  function Header({ next = 8 }: { readonly next?: number }) {
    const [roomId, setRoomId] = useState(7);

    return (
      <>
        <HeaderSearch roomId={roomId} />
        <button type="button" onClick={() => setRoomId(next)}>
          Next room
        </button>
      </>
    );
  }

  it("says what it searches while empty", async () => {
    seedRooms();
    await renderRouted(() => <HeaderSearch roomId={7} />);
    expect(screen.getByText("Search Messages")).toBeTruthy();
  });

  it("starts in the channel on focus, caret after it, and searches it by id", async () => {
    seedRooms();

    const user = userEvent.setup();
    const router = await renderRouted(() => <HeaderSearch roomId={7} />);

    await user.click(input());
    expect(input().value).toBe("in:#launch-planning ");
    expect(input().selectionStart).toBe("in:#launch-planning ".length);

    await user.keyboard("launch{Escape}{Enter}");
    expect(router.state.location.pathname).toBe("/search");
    expect(router.state.location.search).toEqual({ q: "in_id:7 launch" });
  });

  it("names a DM by its person and your notes as you", async () => {
    seedRooms();

    const user = userEvent.setup();

    await renderRouted(() => <Header next={40} />);
    await user.click(screen.getByRole("button", { name: "Next room" }));
    await user.click(input());
    expect(input().value).toBe("in:@me ");
  });

  it("submits an edited filter as typed", async () => {
    seedRooms();

    const user = userEvent.setup();
    const router = await renderRouted(() => <HeaderSearch roomId={9} />);

    await user.click(input());
    expect(input().value).toBe("in:@grace-hopper ");
    await user.keyboard("{Backspace}{Backspace} launch{Escape}{Enter}");
    expect(router.state.location.search).toEqual({ q: "in:@grace-hoppe launch" });
  });

  it("doesn't put the filter back once deleted, and searches everywhere", async () => {
    seedRooms();

    const user = userEvent.setup();
    const router = await renderRouted(() => <HeaderSearch roomId={7} />);

    await user.click(input());
    await user.clear(input());
    expect(input().value).toBe("");
    await user.keyboard("launch");
    expect(input().value).toBe("launch");

    await user.keyboard("{Escape}{Enter}");
    expect(router.state.location.search).toEqual({ q: "launch" });
  });

  it("empties an untouched filter on blur and fills it again on the next focus", async () => {
    seedRooms();

    const user = userEvent.setup();

    await renderRouted(() => <Header />);
    await user.click(input());
    expect(input().value).toBe("in:#launch-planning ");

    await user.tab();
    expect(input().value).toBe("");

    await user.click(input());
    expect(input().value).toBe("in:#launch-planning ");
  });

  it("never carries one room's filter to the next", async () => {
    seedRooms();

    const user = userEvent.setup();
    const router = await renderRouted(() => <Header />);

    await user.click(input());
    await user.keyboard("launch");
    expect(input().value).toBe("in:#launch-planning launch");

    await user.click(screen.getByRole("button", { name: "Next room" }));
    expect(input().value).toBe("");

    await user.click(input());
    expect(input().value).toBe("in:#launch-ops ");
    await user.keyboard("{Enter}");
    expect(router.state.location.search).toEqual({ q: "in_id:8" });
  });
});
