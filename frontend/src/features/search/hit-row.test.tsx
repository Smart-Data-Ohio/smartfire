import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import type { MessageDTO } from "../../store/model.ts";
import { messageFixture } from "../threads/test-fixtures.ts";
import { HitRow } from "./hit-row.tsx";

afterEach(() => {
  cleanup();
  delete document.documentElement.dataset.motion;
});

const WAVE = {
  kind: "available",
  id: 1,
  name: "Wave",
  imageUrl: "/stickers/1",
  stillUrl: "/stickers/1",
  animated: false,
} as const;

const DANCE = {
  kind: "available",
  id: 2,
  name: "Dance",
  imageUrl: "/stickers/2",
  stillUrl: "/stickers/2?still=1",
  animated: true,
} as const;

async function renderHit(hit: MessageDTO, terms: readonly string[] = []) {
  const rootRoute = createRootRoute({
    component: () => (
      <HitRow
        hit={hit}
        conversation={undefined}
        terms={terms}
        now={Date.parse("2026-10-06T16:00:00.000Z")}
      />
    ),
  });

  const router = createRouter({
    routeTree: rootRoute.addChildren([
      createRoute({ getParentRoute: () => rootRoute, path: "/r/$roomId/m/$messageId" }),
      createRoute({
        getParentRoute: () => rootRoute,
        path: "/r/$roomId/t/$threadId",
        validateSearch: () => ({}),
      }),
    ]),
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });

  const result = render(<RouterProvider router={router} />);

  await act(() => router.load());

  return result;
}

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

describe("a sticker hit", () => {
  it("shows a sticker-only message's sticker instead of a blank card", async () => {
    await renderHit(messageFixture(3, { bodyHtml: "", markdownSource: "", sticker: WAVE }), [
      "wave",
    ]);

    const image = screen.getByRole("img", { name: "Sticker: Wave" });

    expect(image.getAttribute("src")).toBe("/stickers/1");
    expect(image.getAttribute("width")).toBe("160");
  });

  it("shows a captioned sticker under its marked caption", async () => {
    const { container } = await renderHit(
      messageFixture(3, { bodyHtml: "<p>hi all</p>", markdownSource: "hi all", sticker: WAVE }),
      ["hi"],
    );

    expect(container.querySelector(".search-hit-body mark")?.textContent).toBe("hi");
    expect(screen.getByRole("img", { name: "Sticker: Wave" })).toBeTruthy();
  });

  it("rests an animated sticker on its first frame under reduced motion", async () => {
    document.documentElement.dataset.motion = "reduce";

    await renderHit(messageFixture(3, { bodyHtml: "", markdownSource: "", sticker: DANCE }));

    expect(screen.getByRole("img", { name: "Sticker: Dance" }).getAttribute("src")).toBe(
      "/stickers/2?still=1",
    );
  });

  it("leaves the deleted-sticker placeholder with its name", async () => {
    const { container } = await renderHit(
      messageFixture(3, {
        bodyHtml: "",
        markdownSource: "",
        sticker: { kind: "deleted", name: "Dance" },
      }),
    );

    expect(container.textContent).toContain("Dance · Sticker deleted");
    expect(screen.queryByRole("img", { name: "Sticker: Dance" })).toBeNull();
  });
});
