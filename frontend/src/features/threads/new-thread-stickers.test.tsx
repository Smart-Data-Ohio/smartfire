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
import { removeToast, toastSnapshot } from "../../ui/toast-store.ts";
import { clearCommandCache } from "../composer/autocomplete/suggestions.ts";
import { resetStickers } from "../composer/sticker-store.ts";
import { NewThreadPane } from "./new-thread-pane.tsx";

const ROOM = SEED_IDS.rooms.quiet;

let network: MockNetwork;

beforeEach(() => {
  for (const toast of toastSnapshot()) removeToast(toast.id);

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
    const response = await fetch(`/api/v1/threads/${threadId}/messages`);
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
  const response = await fetch(`/api/v1/threads/${threadId}/messages`);
  expect(response.status).toBe(200);
  expect(await response.json()).toMatchObject({
    messages: [{ sticker: { kind: "available", id: 1, name: "Wave" } }],
  });
});

it("holds the selected sticker while a queued creation waits for an upload", async () => {
  const { router } = await mountPane();

  await pickSticker("Wave");
  network.server.holdUploads(true);

  const file = document.querySelector<HTMLInputElement>('input[type="file"]');

  if (file === null) throw new Error("No upload input");

  fireEvent.change(file, {
    target: { files: [new File(["one"], "one.txt", { type: "text/plain" })] },
  });
  await waitFor(() => expect(network.server.pendingUploads()).toBe(1));
  fireEvent.click(screen.getByRole("button", { name: "Send message" }));

  const remove = screen.getByRole("button", { name: "Remove Wave sticker" });
  const picker = screen.getByRole("button", { name: "Sticker" });

  await waitFor(() => expect(remove).toHaveProperty("disabled", true));
  expect(picker).toHaveProperty("disabled", true);
  fireEvent.click(remove);
  fireEvent.click(picker);
  expect(screen.getByRole("img", { name: "Sticker: Wave" }).getAttribute("src")).toBe(
    "/stickers/1",
  );
  expect(screen.queryByRole("dialog", { name: "Send a sticker" })).toBeNull();

  await act(async () => network.server.releaseUploads());
  await waitFor(() =>
    expect(router.state.location.pathname).toMatch(new RegExp(`/r/${ROOM}/t/\\d+$`)),
  );

  const threadId = Number(router.state.location.pathname.split("/").at(-1));
  const response = await fetch(`/api/v1/threads/${threadId}/messages`);

  expect(response.status).toBe(200);
  expect(await response.json()).toMatchObject({
    messages: [
      {
        sticker: { kind: "available", id: 1, name: "Wave" },
        attachment: { filename: "one.txt" },
      },
    ],
  });
});

it("retains the caption and sticker after a failed creation, then retries with another sticker", async () => {
  const { parent, router } = await mountPane();
  const input = screen.getByRole("textbox", { name: "Reply…" });

  fireEvent.change(input, { target: { value: "Retry caption" } });
  await pickSticker("Dance");

  const removed = await fetch("/api/v1/admin/stickers/2", {
    method: "DELETE",
    headers: { "X-CSRF-Token": network.server.csrfToken() },
  });

  expect(removed.status).toBe(200);
  fireEvent.click(screen.getByRole("button", { name: "Send message" }));
  await waitFor(() => expect(toastSnapshot().at(-1)?.title).toBe("Couldn't start the thread"));
  await waitFor(() =>
    expect(screen.getByRole("button", { name: "Send message" })).toHaveProperty("disabled", false),
  );
  expect(router.state.location.pathname).toBe("/");
  expect(store.getState().messages[parent.id]?.thread).toBeNull();
  expect(input).toHaveProperty("value", "Retry caption");
  expect(screen.getByRole("img", { name: "Sticker: Dance" }).getAttribute("src")).toBe(
    "/stickers/2?still=1",
  );

  await pickSticker("Wave");
  fireEvent.click(screen.getByRole("button", { name: "Send message" }));
  await waitFor(() =>
    expect(router.state.location.pathname).toMatch(new RegExp(`/r/${ROOM}/t/\\d+$`)),
  );

  const threadId = Number(router.state.location.pathname.split("/").at(-1));
  const response = await fetch(`/api/v1/threads/${threadId}/messages`);

  expect(response.status).toBe(200);
  expect(await response.json()).toMatchObject({
    messages: [
      { markdownSource: "Retry caption", sticker: { kind: "available", id: 1, name: "Wave" } },
    ],
  });
});
