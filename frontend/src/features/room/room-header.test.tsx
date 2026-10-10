import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { roomDetailFixture } from "../../api/testing.ts";
import { beginRoomRequest } from "../../store/join-state.ts";
import { mutations } from "../../store/store.ts";
import { DetailsPane } from "../panes/details-pane.tsx";
import { openPane, useOpenPane } from "../panes/pane-store.ts";
import { RoomHeader } from "./room-header.tsx";

async function mount(topic: string | null, phone = false) {
  vi.stubGlobal("matchMedia", (media: string) =>
    Object.assign(new EventTarget(), {
      matches: phone,
      media,
      onchange: null,
      addListener: () => undefined,
      removeListener: () => undefined,
    }),
  );
  const detail = roomDetailFixture(5);
  detail.room.topic = topic;
  mutations.setRoomDetail(detail, beginRoomRequest());

  function Harness() {
    const pane = useOpenPane(5);

    return (
      <>
        <RoomHeader roomId={5} />
        {pane === "details" ? <DetailsPane roomId={5} /> : null}
      </>
    );
  }

  const root = createRootRoute();
  const room = createRoute({ getParentRoute: () => root, path: "/r/$roomId", component: Harness });

  const router = createRouter({
    routeTree: root.addChildren([room]),
    history: createMemoryHistory({ initialEntries: ["/r/5"] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());
}

afterEach(() => {
  openPane(null);
  mutations.reset();
  vi.unstubAllGlobals();
});

describe("channel topic header", () => {
  it("uses the single-line truncated topic style and opens the full plain-text topic in About", async () => {
    const topic =
      "<img src=x onerror=alert(1)> Plans https://example.com/path?x=1&y=2.\n" +
      "More plans ".repeat(70);

    await mount(topic);
    const button = screen.getByRole("button", { name: "Channel topic, About" });
    expect(button.classList.contains("room-header-topic")).toBe(true);
    expect(button.textContent).toBe(topic);
    await userEvent.setup().click(button);
    const about = screen.getByRole("region", { name: "About" });
    expect(about.textContent).toContain(topic);
    expect(about.querySelector("img")).toBeNull();
    expect(
      screen.getByRole("link", { name: "https://example.com/path?x=1&y=2" }).getAttribute("href"),
    ).toBe("https://example.com/path?x=1&y=2");
  });

  it("hides the topic on phones but keeps it in About", async () => {
    await mount("Phone topic", true);
    expect(screen.queryByRole("button", { name: "Channel topic, About" })).toBeNull();
    await userEvent.setup().click(screen.getByRole("button", { name: "room-5, details" }));
    expect(screen.getByRole("region", { name: "About" }).textContent).toContain("Phone topic");
  });

  it("omits the topic button when no topic is set", async () => {
    await mount(null);
    expect(screen.queryByRole("button", { name: "Channel topic, About" })).toBeNull();
  });
});
