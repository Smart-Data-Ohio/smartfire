/** Test support for the admin sections: a section in a memory router under `/app/`, as it mounts. */
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render } from "@testing-library/react";
import type { ReactNode } from "react";

/**
 * Renders `ui` at `/app/admin/<path>` inside a memory router, so the section's links resolve.
 * Answers the router's path below its `/app` base, for a test that follows a link.
 */
export async function renderInRouter(
  ui: ReactNode,
  path = "workspace",
): Promise<{ pathname(): string }> {
  // The root renders the section, so following one of its links keeps it mounted.
  const root = createRootRoute({ component: () => ui });
  const section = createRoute({ getParentRoute: () => root, path: "admin/$section" });

  const router = createRouter({
    routeTree: root.addChildren([section]),
    basepath: "/app",
    history: createMemoryHistory({ initialEntries: [`/app/admin/${path}`] }),
    defaultNotFoundComponent: () => null,
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());

  return { pathname: () => router.state.location.pathname };
}
