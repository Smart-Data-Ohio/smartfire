import {
  createMemoryHistory,
  createRootRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { userFixture } from "../../api/testing.ts";
import type { User } from "../../store/model.ts";
import { mutations } from "../../store/store.ts";
import { peoplePages } from "../../sync/admin.ts";
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
) {
  vi.spyOn(peoplePages, "profile").mockResolvedValue({
    user: { ...userFixture(2, name), accountName, pronouns, ...banner },
    status: null,
    dndAllowed: null,
    emailAddress: null,
    transferUrl: null,
    transferQrSvg: null,
    canBan: false,
    ignoredByMe: false,
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
