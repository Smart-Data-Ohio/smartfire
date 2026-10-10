import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, cleanup, render } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { messageFixture } from "../threads/test-fixtures.ts";
import { HitRow } from "./hit-row.tsx";

afterEach(() => {
  cleanup();
  delete document.documentElement.dataset.motion;
});

it("shows an animated workspace icon's first frame under reduced motion, marks and all", async () => {
  document.documentElement.dataset.motion = "reduce";

  const hit = messageFixture(3, {
    bodyHtml:
      '<p>dance <img class="icon icon--custom" src="/icons/dance" alt=":dance:" title="Dance" draggable="false"></p>',
  });

  const rootRoute = createRootRoute({
    component: () => (
      <HitRow
        hit={hit}
        conversation={undefined}
        terms={["dance"]}
        now={Date.parse("2026-10-06T16:00:00.000Z")}
      />
    ),
  });

  const router = createRouter({
    routeTree: rootRoute.addChildren([
      createRoute({ getParentRoute: () => rootRoute, path: "/r/$roomId/m/$messageId" }),
    ]),
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });

  const { container } = render(<RouterProvider router={router} />);

  await act(() => router.load());

  expect(container.querySelector(".search-hit-body mark")?.textContent).toBe("dance");
  expect(container.querySelector(".search-hit-body img")?.getAttribute("src")).toBe(
    "/icons/dance?still=1",
  );
});
