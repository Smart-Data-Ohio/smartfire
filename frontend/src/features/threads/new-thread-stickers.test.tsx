import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { SEED_IDS } from "../../../mock/server.ts";
import { mutations, store } from "../../store/store.ts";
import { installMockNetwork, type MockNetwork } from "../../test/mock-network.ts";
import { clearCommandCache } from "../composer/autocomplete/suggestions.ts";
import { resetStickers } from "../composer/sticker-store.ts";
import { NewThreadPane } from "./new-thread-pane.tsx";

const ROOM = SEED_IDS.rooms.quiet;
let network: MockNetwork;

beforeEach(() => {
  network = installMockNetwork();
  mutations.reset();
  resetStickers();
  clearCommandCache();
  sessionStorage.clear();
  document.documentElement.dataset.motion = "reduce";
  vi.stubGlobal("matchMedia", (media: string) =>
    Object.assign(new EventTarget(), { media, matches: false, onchange: null }),
  );
  Element.prototype.scrollIntoView = () => undefined;
  const meta = document.createElement("meta");
  meta.name = "csrf-token";
  meta.content = network.server.csrfToken();
  document.head.append(meta);
});

afterEach(() => {
  network.restore();
  mutations.reset();
  resetStickers();
  vi.unstubAllGlobals();
  sessionStorage.clear();
  delete document.documentElement.dataset.motion;
  document.head.querySelector('meta[name="csrf-token"]')?.remove();
});

async function mountPane() {
  const parent = network.server.post(ROOM, SEED_IDS.viewer, "Thread parent");
  store.setState({ messages: { [parent.id]: parent } });
  const root = createRootRoute({
    component: () => <NewThreadPane roomId={ROOM} parentId={parent.id} />,
  });
  const router = createRouter({
    routeTree: root.addChildren([
      createRoute({
        getParentRoute: () => root,
        path: "/r/$roomId/t/$threadId",
        component: () => <p>Thread opened</p>,
      }),
    ]),
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });
  render(<RouterProvider router={router} />);
  await act(() => router.load());
  return { parent, router };
}

async function pickSticker(name: string) {
  fireEvent.click(await screen.findByRole("button", { name: "Sticker" }));
  const picker = await screen.findByRole("dialog", { name: "Send a sticker" });
  fireEvent.click(await within(picker).findByRole("option", { name }));
  await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
}

it.each([
  { caption: "", name: "Wave", id: 1 },
  { caption: "hi all", name: "Dance", id: 2 },
])(
  "starts a thread with $name and caption '$caption' through the API",
  async ({ caption, name, id }) => {
    const { parent, router } = await mountPane();
    const input = screen.getByRole("textbox", { name: "Reply…" });
    fireEvent.change(input, { target: { value: caption } });
    await pickSticker(name);
    expect(store.getState().messages[parent.id]?.thread).toBeNull();
    fireEvent.keyDown(input, { key: "Enter" });
    await waitFor(() =>
      expect(router.state.location.pathname).toMatch(new RegExp(`/r/${ROOM}/t/\\d+$`)),
    );
    const threadId = Number(router.state.location.pathname.split("/").at(-1));
    const response = await fetch(`/api/v1/threads/${threadId}/replies`);
    expect(response.status).toBe(200);
    expect(await response.json()).toMatchObject({
      messages: [{ markdownSource: caption, threadId, sticker: { kind: "available", id, name } }],
    });
  },
);

it("removes and replaces a held sticker before sending", async () => {
  const { router } = await mountPane();
  await pickSticker("Dance");
  expect(screen.getByRole("img", { name: "Sticker: Dance" }).getAttribute("src")).toBe(
    "/stickers/2?still=1",
  );
  fireEvent.click(screen.getByRole("button", { name: "Remove Dance sticker" }));
  expect(screen.getByRole("button", { name: "Send message" })).toHaveProperty("disabled", true);
  await pickSticker("Dance");
  await pickSticker("Wave");
  fireEvent.click(screen.getByRole("button", { name: "Send message" }));
  await waitFor(() =>
    expect(router.state.location.pathname).toMatch(new RegExp(`/r/${ROOM}/t/\\d+$`)),
  );
  const threadId = Number(router.state.location.pathname.split("/").at(-1));
  const response = await fetch(`/api/v1/threads/${threadId}/replies`);
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchObject({
    messages: [{ sticker: { kind: "available", id: 1, name: "Wave" } }],
  });
});
