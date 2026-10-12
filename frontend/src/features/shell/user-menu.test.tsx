import {
  createMemoryHistory,
  createRootRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SHEET_QUERY } from "../../ui/action-sheet.tsx";
import { tour } from "../help/tour.tsx";
import { overlayStore } from "../switcher/overlay-store.ts";
import { UserMenu } from "./user-menu.tsx";

/** The account menu under `/app`, opened from a plain button. */
async function openMenu() {
  const root = createRootRoute({
    component: () => (
      <UserMenu
        trigger={(props) => (
          <button {...props} type="button">
            Your account
          </button>
        )}
      />
    ),
  });

  const router = createRouter({
    routeTree: root,
    basepath: "/app",
    history: createMemoryHistory({ initialEntries: ["/app/"] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());
  fireEvent.click(screen.getByRole("button", { name: "Your account" }));

  return screen.getByRole("menu", { name: "Your account" });
}

/** The Help group's items, by name. */
function helpItems(menu: HTMLElement): readonly string[] {
  return within(within(menu).getByRole("group", { name: "Help" }))
    .getAllByRole("menuitem")
    .map((item) => item.querySelector(".menu-item-label")?.textContent ?? "");
}

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  tour.store.setState({ open: false, index: 0 });
  overlayStore.setState({ open: null });
  Reflect.deleteProperty(window, "matchMedia");
});

describe("the account menu's Help items", () => {
  it("offers the shortcuts, the tour and the public pages", async () => {
    const menu = await openMenu();

    expect(helpItems(menu)).toEqual([
      "Keyboard shortcuts",
      "Restart tour",
      "About",
      "Privacy Policy",
      "Terms of Service",
    ]);
  });

  it("opens the shortcuts sheet", async () => {
    const menu = await openMenu();

    fireEvent.click(within(menu).getByRole("menuitem", { name: /Keyboard shortcuts/ }));

    expect(overlayStore.getState().open).toBe("shortcuts");
  });

  it("restarts the tour from its first step", async () => {
    tour.store.setState({ open: false, index: 3 });

    const menu = await openMenu();

    fireEvent.click(within(menu).getByRole("menuitem", { name: "Restart tour" }));

    expect(tour.store.getState()).toEqual({ open: true, index: 0 });
  });

  it.each([
    ["About", "/app/about"],
    ["Privacy Policy", "/app/privacy"],
    ["Terms of Service", "/app/terms"],
  ])("opens %s in a new tab, keeping the conversation", async (name, path) => {
    const open = vi.spyOn(window, "open").mockReturnValue(null);
    const menu = await openMenu();

    fireEvent.click(within(menu).getByRole("menuitem", { name }));

    expect(open).toHaveBeenCalledWith(path, "_blank", "noopener");
    await waitFor(() => expect(screen.queryByRole("menu")).toBeNull());
  });

  it("holds the same items in the phone's sheet, from the You tab", async () => {
    window.matchMedia = (query: string) =>
      Object.assign(new EventTarget(), {
        matches: query === SHEET_QUERY,
        media: query,
        onchange: null,
        addListener: () => undefined,
        removeListener: () => undefined,
      });

    const menu = await openMenu();

    expect(menu.classList.contains("action-sheet")).toBe(true);
    expect(helpItems(menu)).toEqual([
      "Keyboard shortcuts",
      "Restart tour",
      "About",
      "Privacy Policy",
      "Terms of Service",
    ]);
  });
});
