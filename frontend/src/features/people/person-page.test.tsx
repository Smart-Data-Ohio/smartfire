import {
  createMemoryHistory,
  createRootRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { userFixture } from "../../api/testing.ts";
import { mutations } from "../../store/store.ts";
import { peoplePages } from "../../sync/admin.ts";
import { PersonPage } from "./person-page.tsx";

afterEach(() => {
  vi.restoreAllMocks();
  mutations.reset();
});

async function mount(name: string, accountName: string, pronouns: string | null) {
  vi.spyOn(peoplePages, "profile").mockResolvedValue({
    user: { ...userFixture(2, name), accountName, pronouns },
    status: null,
    dndAllowed: null,
    emailAddress: null,
    transferUrl: null,
    transferQrSvg: null,
    canBan: false,
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
