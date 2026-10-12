import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it } from "vitest";
import { NotFound } from "./not-found.tsx";

it("keeps an unknown route in the SPA and offers a working home link", async () => {
  const root = createRootRoute({ notFoundComponent: NotFound });

  const home = createRoute({
    getParentRoute: () => root,
    path: "/",
    component: () => <p>Home destination</p>,
  });

  const router = createRouter({
    routeTree: root.addChildren([home]),
    basepath: "/app",
    history: createMemoryHistory({ initialEntries: ["/app/missing?classic=1"] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());
  expect(screen.getByRole("region", { name: "Page not found" })).toBeTruthy();
  expect(screen.getAllByRole("link").map((link) => link.getAttribute("href"))).toEqual(["/app/"]);
  await userEvent.setup().click(screen.getByRole("link", { name: "Go home" }));
  expect(await screen.findByText("Home destination")).toBeTruthy();
  expect(router.history.location.href).toBe("/app/");
});
