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
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { roomDetailFixture } from "../../api/testing.ts";
import type { RoomKind } from "../../gen/RoomKind.ts";
import { PHONE_QUERY } from "../../lib/breakpoints.ts";
import { initialState } from "../../store/state.ts";
import { store } from "../../store/store.ts";
import { type CallState, callStore, initialCallState } from "./call-store.ts";
import { CallView } from "./call-view.tsx";
import type { CallParticipant } from "./engine/transport.ts";
import {
  setTheaterChat,
  THEATER_CHAT_STORAGE_KEY,
  theaterChatStore,
  theaterIn,
} from "./theater-chat.ts";

const ROOM = 8;

function participant(identity: string, local: boolean, screenId: string | null): CallParticipant {
  return {
    identity,
    name: identity,
    local,
    speaking: false,
    audioLevel: 0,
    microphoneMuted: false,
    quality: "good",
    cameraId: null,
    screenId,
  };
}

const ME = participant("me", true, null);

const MAYA = participant("maya", false, "screen-maya");

/** In `ROOM`'s call with Maya sharing, her share expanded unless `expanded` says otherwise. */
function inCall(expanded: string | null = "screen-maya"): CallState {
  return {
    ...initialCallState,
    phase: "connected",
    roomId: ROOM,
    roomName: "Lounge",
    viewOpen: true,
    expandedVideoId: expanded,
    snapshot: { ...initialCallState.snapshot, participants: [ME, MAYA] },
  };
}

function roomOfKind(kind: RoomKind): void {
  const detail = roomDetailFixture(ROOM);

  store.setState({
    rooms: {
      [ROOM]: {
        detail: { ...detail, room: { ...detail.room, kind } },
        status: "ready",
        error: null,
        preview: null,
      },
    },
  });
}

/** A desktop window, or a phone's (the phone layout's media query matches). */
function screenWidth(phone: boolean): void {
  vi.stubGlobal("matchMedia", (query: string) =>
    Object.assign(new EventTarget(), {
      matches: phone && query === PHONE_QUERY,
      media: query,
      onchange: null,
      addListener: () => undefined,
      removeListener: () => undefined,
    }),
  );
}

async function mount(path = `/app/r/${ROOM}`): Promise<void> {
  const root = createRootRoute({ component: Outlet });

  const room = createRoute({
    getParentRoute: () => root,
    path: "/r/$roomId",
    component: () => <CallView roomId={ROOM} />,
  });

  const router = createRouter({
    routeTree: root.addChildren([room]),
    basepath: "/app",
    history: createMemoryHistory({ initialEntries: [path] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());
}

function callView(): HTMLElement {
  return screen.getByRole("region", { name: "Call" });
}

beforeEach(() => {
  screenWidth(false);
  sessionStorage.clear();
  theaterChatStore.setState({ open: true });
});

afterEach(() => {
  vi.unstubAllGlobals();
  callStore.setState(initialCallState, true);
  store.setState(initialState, true);
});

describe("theaterIn", () => {
  it("is the call's room with a share expanded that is still in the call", () => {
    expect(theaterIn(inCall(), ROOM)).toBe(true);
    expect(theaterIn(inCall(), ROOM + 1)).toBe(false);
    expect(theaterIn(inCall(null), ROOM)).toBe(false);
    expect(theaterIn(inCall("screen-gone"), ROOM)).toBe(false);
    expect(theaterIn({ ...inCall(), phase: "failed" }, ROOM)).toBe(false);
  });
});

describe("setTheaterChat", () => {
  it("remembers the choice for the tab's session", () => {
    setTheaterChat(false);
    expect(theaterChatStore.getState().open).toBe(false);
    expect(sessionStorage.getItem(THEATER_CHAT_STORAGE_KEY)).toBe("hidden");

    setTheaterChat(true);
    expect(theaterChatStore.getState().open).toBe(true);
    expect(sessionStorage.getItem(THEATER_CHAT_STORAGE_KEY)).toBe("shown");
  });
});

describe("the chat beside theater mode", () => {
  it("shows by default and hides and shows again from the share's bar", async () => {
    roomOfKind("open");
    callStore.setState(inCall(), true);
    await mount();

    expect(callView().dataset.theater).toBe("true");
    expect(callView().dataset.chat).toBe("true");

    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "Hide chat" }));
    expect(callView().dataset.chat).toBeUndefined();
    expect(callView().dataset.theater).toBe("true");
    expect(sessionStorage.getItem(THEATER_CHAT_STORAGE_KEY)).toBe("hidden");

    await user.click(screen.getByRole("button", { name: "Show chat" }));
    expect(callView().dataset.chat).toBe("true");
  });

  it("is a direct message's chat as much as a channel's", async () => {
    roomOfKind("direct");
    callStore.setState(inCall(), true);
    await mount();

    expect(callView().dataset.chat).toBe("true");
    expect(screen.getByRole("button", { name: "Hide chat" })).toBeTruthy();
  });

  it("only comes with theater mode", async () => {
    roomOfKind("open");
    callStore.setState(inCall(null), true);
    await mount();

    expect(callView().dataset.chat).toBeUndefined();
    expect(screen.queryByRole("button", { name: /chat$/ })).toBeNull();
  });

  it("isn't offered in a board, which has no conversation", async () => {
    roomOfKind("board");
    callStore.setState(inCall(), true);
    await mount();

    expect(callView().dataset.theater).toBe("true");
    expect(callView().dataset.chat).toBeUndefined();
    expect(screen.queryByRole("button", { name: /chat$/ })).toBeNull();
  });

  it("shares a phone's full-screen call view with the chat under the share", async () => {
    screenWidth(true);
    roomOfKind("open");
    // A phone's view follows the URL's `call=1`, whatever `viewOpen` says.
    callStore.setState({ ...inCall(), viewOpen: false }, true);
    await mount(`/app/r/${ROOM}?call=1`);

    expect(callView().dataset.chat).toBe("true");

    await userEvent.setup().click(screen.getByRole("button", { name: "Hide chat" }));
    expect(callView().dataset.chat).toBeUndefined();
  });
});
