import {
  createMemoryHistory,
  createRootRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { userFixture } from "../../api/testing.ts";
import type { User } from "../../store/model.ts";
import { mutations, store } from "../../store/store.ts";
import { ignores, peoplePages } from "../../sync/admin.ts";
import { PersonPage } from "./person-page.tsx";

afterEach(() => {
  vi.restoreAllMocks();
  mutations.reset();
});

async function mount(
  name: string,
  accountName: string,
  pronouns: string | null,
  banner: Partial<User> = {},
  ignoredByMe = false,
) {
  vi.spyOn(peoplePages, "profile").mockResolvedValue({
    user: { ...userFixture(2, name), accountName, pronouns, ...banner },
    status: null,
    dndAllowed: null,
    emailAddress: null,
    transferUrl: null,
    transferQrSvg: null,
    canBan: false,
    ignoredByMe,
    canManageBot: false,
  });

  const root = createRootRoute({ component: () => <PersonPage userId={2} viewerId={1} /> });

  const router = createRouter({
    routeTree: root,
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());
}

/** Answers an ignore write as the server would, landing it in the store as the facade does. */
function answerIgnores() {
  let revision = 10;

  return vi.spyOn(ignores, "set").mockImplementation((userId, ignored) => {
    revision += 1;

    const change = { userId, ignoredByMe: ignored, revision };

    mutations.applyBlockState(change);

    return Promise.resolve(change);
  });
}

it("shows the nickname, muted pronouns and secondary account name", async () => {
  await mount("Maya", "Maya Okafor", "she/her");
  expect(await screen.findByRole("heading", { name: "Maya" })).toBeTruthy();
  expect(screen.getByText("she/her").classList.contains("text-muted")).toBe(true);
  expect(screen.getByText("Maya Okafor").classList.contains("text-muted")).toBe(true);
  expect(screen.getByRole("button", { name: "Message Maya" })).toBeTruthy();
});

it("shows the account name once when no nickname or pronouns are set", async () => {
  await mount("Maya Okafor", "Maya Okafor", null);
  expect(await screen.findByRole("heading", { name: "Maya Okafor" })).toBeTruthy();
  expect(screen.getAllByText("Maya Okafor")).toHaveLength(1);
  expect(screen.queryByText("she/her")).toBeNull();
});

it("shows their banner above the avatar, and a new one as soon as it syncs", async () => {
  const first = "/users/token/banner?v=1";

  await mount("Maya", "Maya", null, { bannerUrl: first, bannerStillUrl: null });

  const banner = await screen.findByRole("img", { name: "Maya's profile banner" });

  expect(banner.getAttribute("src")).toBe(first);
  expect(banner.closest(".profile-hero")?.querySelector(".profile-hero-avatar")).toBeTruthy();

  act(() =>
    mutations.mergeUsers([{ ...userFixture(2, "Maya"), bannerUrl: "/users/token/banner?v=2" }]),
  );
  expect(screen.getByRole("img", { name: "Maya's profile banner" }).getAttribute("src")).toBe(
    "/users/token/banner?v=2",
  );
});

it("shows the avatar's tint where someone has no banner", async () => {
  await mount("Maya", "Maya", null);
  await screen.findByRole("heading", { name: "Maya" });
  expect(screen.queryByRole("img", { name: "Maya's profile banner" })).toBeNull();
  expect(document.querySelector<HTMLElement>(".person .profile-banner")?.dataset.empty).toBe(
    "true",
  );
});

it("ignores someone after a confirm that says what it does, and stops without one", async () => {
  await mount("Maya", "Maya", null);

  const set = answerIgnores();

  fireEvent.click(await screen.findByRole("button", { name: "Ignore" }));

  const dialog = await screen.findByRole("alertdialog", { name: "Ignore Maya?" });

  expect(dialog.textContent).toContain("Their messages will be collapsed for you");
  expect(dialog.textContent).toContain("won't be able to start new DMs with you");
  expect(set).not.toHaveBeenCalled();

  fireEvent.click(screen.getByRole("button", { name: "Ignore Maya" }));
  expect(set).toHaveBeenCalledWith(2, true);
  expect(await screen.findByRole("button", { name: "Stop ignoring" })).toBeTruthy();
  expect(screen.getByText("Ignored")).toBeTruthy();

  fireEvent.click(screen.getByRole("button", { name: "Stop ignoring" }));
  expect(set).toHaveBeenLastCalledWith(2, false);
  await waitFor(() => expect(screen.getByRole("button", { name: "Ignore" })).toBeTruthy());
  expect(screen.queryByText("Ignored")).toBeNull();
  expect(screen.queryByRole("alertdialog", { name: "Ignore Maya?" })).toBeNull();
});

it("follows an ignore made in another session", async () => {
  await mount("Maya", "Maya", null);
  await screen.findByRole("button", { name: "Ignore" });

  act(() => mutations.applyBlockState({ userId: 2, ignoredByMe: true, revision: 3 }));

  expect(screen.getByRole("button", { name: "Stop ignoring" })).toBeTruthy();
  expect(store.getState().ignores.people[2]).toBe(true);
});

it("follows an unignore made in another session once the snapshot drops them", async () => {
  mutations.setBlocks({
    revision: 4,
    people: [
      { user: userFixture(2, "Maya"), ignoredByMe: true, createdAt: "2026-10-01T12:00:00Z" },
    ],
  });
  await mount("Maya", "Maya", null, {}, true);
  expect(await screen.findByRole("button", { name: "Stop ignoring" })).toBeTruthy();

  // The other session's write echoes here as a refetch of the list, which no longer has Maya.
  act(() => mutations.setBlocks({ revision: 5, people: [] }));

  expect(screen.getByRole("button", { name: "Ignore" })).toBeTruthy();
  expect(screen.queryByText("Ignored")).toBeNull();
});
