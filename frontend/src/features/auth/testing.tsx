/**
 * Test support for the signed-out pages: the four routes in a memory router under `/app/`, as
 * src/router.tsx mounts them, and the mock's sign-in controls.
 */
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render } from "@testing-library/react";
import type { MockNetwork } from "../../test/mock-network.ts";
import { ChallengePage } from "./challenge.tsx";
import { FirstRunPage } from "./first-run.tsx";
import { SignInPage } from "./sign-in.tsx";
import { TransferRoute } from "./transfer.tsx";

/** Where the memory router is, under its `/app` base. */
export interface AuthRouter {
  pathname(): string;
}

/** Renders the signed-out pages at `path` (e.g. `/app/session/new`). */
export async function renderAuth(path: string): Promise<AuthRouter> {
  const root = createRootRoute({ component: Outlet });

  const routes = [
    createRoute({ getParentRoute: () => root, path: "session/new", component: SignInPage }),
    createRoute({
      getParentRoute: () => root,
      path: "two_factor/challenge",
      component: ChallengePage,
    }),
    createRoute({
      getParentRoute: () => root,
      path: "session/transfers/$transferId",
      component: TransferRoute,
    }),
    createRoute({ getParentRoute: () => root, path: "first_run", component: FirstRunPage }),
  ];

  const router = createRouter({
    routeTree: root.addChildren(routes),
    basepath: "/app",
    history: createMemoryHistory({ initialEntries: [path] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());

  return { pathname: () => router.state.location.pathname };
}

/** The mock's sign-in state: Google configured, first run pending, a second step waiting. */
export interface SignInChange {
  readonly google?: boolean;
  readonly firstRunPending?: boolean;
  readonly pending?: boolean;
  readonly firstRunUnavailable?: boolean;
}

/** Starts the mock over, then applies `change` to its sign-in state. */
export async function resetSignIn(network: MockNetwork, change: SignInChange = {}): Promise<void> {
  await fetch("/__mock/reset", { method: "POST" });
  await fetch("/__mock/sign-in", {
    method: "POST",
    headers: { "Content-Type": "application/json", "X-CSRF-Token": network.server.csrfToken() },
    body: JSON.stringify(change),
  });
}
