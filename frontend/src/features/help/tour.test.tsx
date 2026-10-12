import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterAll, afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { intField, parseJson } from "../../../mock/json.ts";
import { get, harness } from "../../../mock/s2/testing.ts";
import type { Me } from "../../gen/Me.ts";
import { PHONE_QUERY } from "../../lib/breakpoints.ts";
import { installMockNetwork } from "../../test/mock-network.ts";

const network = installMockNetwork();

afterAll(() => network.restore());

const FIRST = "Your rooms live here";

const LAST = "Shortcuts live under Ctrl+/";

let seed: Me;

/**
 * Answers `query` as a phone would (or not), with nothing ever changing. Set on `window` itself,
 * as `vi.unstubAllGlobals` would also take away the mock network's `fetch`.
 */
function stubMedia(phone: boolean): void {
  Object.assign(window, {
    matchMedia: (query: string) =>
      Object.assign(new EventTarget(), {
        media: query,
        matches: phone && query === PHONE_QUERY,
        onchange: null,
      }),
  });
}

/**
 * The tour and the store from a fresh module graph: the tour starts by itself once per page load,
 * so each test gets its own page load.
 */
async function load(tourCompleted: boolean) {
  vi.resetModules();

  const { mutations } = await import("../../store/store.ts");
  const help = await import("./tour.tsx");

  mutations.reset();
  mutations.setMe({ ...seed, preferences: { ...seed.preferences, tourCompleted } });

  return help;
}

/** How many stamps the mock has had through `PATCH /users/me/tour`. */
async function stamps(): Promise<number | null> {
  return intField(parseJson(await (await fetch("/__mock/state")).text()), "tourStamps");
}

/** An anchor laid out at `box`, as jsdom lays nothing out by itself. */
function anchor(
  className: string,
  box: { top: number; left: number; width: number; height: number },
) {
  const element = document.createElement("div");

  element.className = className;
  element.getBoundingClientRect = () => DOMRect.fromRect({ x: box.left, y: box.top, ...box });
  element.scrollIntoView = () => undefined;
  document.body.append(element);

  return element;
}

const dialog = () => document.querySelector<HTMLDialogElement>("dialog.tour");

beforeEach(async () => {
  seed = await get<Me>(harness().server, "/api/v1/me");
  stubMedia(false);
});

afterEach(() => {
  cleanup();
  document.body.replaceChildren();
  vi.restoreAllMocks();
  Reflect.deleteProperty(window, "matchMedia");
});

describe("the product tour", () => {
  it("starts by itself for someone who hasn't completed it, named and focused on Next", async () => {
    const { ProductTour } = await load(false);

    render(<ProductTour />);

    const card = await screen.findByRole("dialog", { name: FIRST });

    expect(card.getAttribute("aria-modal")).toBe("true");
    expect(screen.getByText("Step 1 of 5")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Back" })).toBeNull();
    await waitFor(() => expect(document.activeElement?.textContent).toBe("Next"));
  });

  it("stays away for someone who has completed it", async () => {
    const { ProductTour } = await load(true);

    render(<ProductTour />);
    await act(async () => undefined);

    expect(dialog()).toBeNull();
  });

  it("skips with Skip tour, stamps completion once and marks /me completed", async () => {
    const { ProductTour } = await load(false);
    const { store } = await import("../../store/store.ts");
    const before = await stamps();

    render(<ProductTour />);
    fireEvent.click(await screen.findByRole("button", { name: "Skip tour" }));

    await waitFor(() => expect(dialog()).toBeNull());
    expect(store.getState().me?.preferences.tourCompleted).toBe(true);
    await waitFor(async () => expect(await stamps()).toBe((before ?? 0) + 1));
  });

  it("skips with Escape and gives focus back to where it was", async () => {
    const { ProductTour, restartTour } = await load(true);
    const opener = document.createElement("button");

    opener.textContent = "Restart tour";
    document.body.append(opener);
    opener.focus();
    render(<ProductTour />);
    act(restartTour);

    const card = await screen.findByRole("dialog", { name: FIRST });

    fireEvent.keyDown(card, { key: "Escape" });

    await waitFor(() => expect(dialog()).toBeNull());
    await waitFor(() => expect(document.activeElement).toBe(opener));
  });

  it("restarts from the first step after it was completed, without stamping on the way in", async () => {
    const { ProductTour, restartTour } = await load(true);
    const before = await stamps();

    render(<ProductTour />);
    act(restartTour);

    expect(await screen.findByRole("dialog", { name: FIRST })).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect(await screen.findByText("Step 2 of 5")).toBeTruthy();

    act(restartTour);
    expect(await screen.findByText("Step 1 of 5")).toBeTruthy();
    expect(await stamps()).toBe(before);
  });

  it("centres every step whose anchor is missing and still runs to Finish", async () => {
    const { ProductTour, restartTour } = await load(true);
    const before = await stamps();

    render(<ProductTour />);
    act(restartTour);
    await screen.findByRole("dialog", { name: FIRST });

    for (let step = 1; step < 5; step += 1) {
      expect(dialog()?.hasAttribute("data-anchored")).toBe(false);
      fireEvent.click(screen.getByRole("button", { name: "Next" }));
      expect(await screen.findByText(`Step ${step + 1} of 5`)).toBeTruthy();
    }

    expect(screen.getByRole("dialog", { name: LAST })).toBeTruthy();
    expect(dialog()?.hasAttribute("data-anchored")).toBe(false);
    expect(document.querySelector<HTMLElement>(".tour-card")?.style.left).toBe("");
    fireEvent.click(screen.getByRole("button", { name: "Finish" }));

    await waitFor(() => expect(dialog()).toBeNull());
    await waitFor(async () => expect(await stamps()).toBe((before ?? 0) + 1));
  });

  it("rings and places the card beside an anchor on a wide screen", async () => {
    const { ProductTour, restartTour } = await load(true);

    anchor("sidebar", { top: 0, left: 0, width: 240, height: 700 });
    render(<ProductTour />);
    act(restartTour);

    await waitFor(() => expect(dialog()?.getAttribute("data-anchored")).toBe(""));
    expect(dialog()?.hasAttribute("data-dock")).toBe(false);
    expect(document.querySelector<HTMLElement>(".tour-spotlight")?.style.width).toBe("248px");
    await waitFor(() =>
      expect(document.querySelector<HTMLElement>(".tour-card")?.style.left).not.toBe(""),
    );
  });

  it("docks the card as a sheet on a phone, away from its anchor", async () => {
    stubMedia(true);

    const { ProductTour, restartTour } = await load(true);

    // The last step's anchor (the You button), at the foot of the screen.
    anchor("sidebar-you-button", { top: 700, left: 300, width: 60, height: 56 });
    render(<ProductTour />);
    act(restartTour);
    await screen.findByRole("dialog", { name: FIRST });

    // A phone's room has no sidebar: the first step has no anchor and isn't docked.
    expect(dialog()?.hasAttribute("data-anchored")).toBe(false);
    expect(dialog()?.hasAttribute("data-dock")).toBe(false);

    for (let step = 1; step < 5; step += 1) {
      fireEvent.click(screen.getByRole("button", { name: "Next" }));
    }

    await screen.findByRole("dialog", { name: LAST });
    await waitFor(() => expect(dialog()?.getAttribute("data-dock")).toBe("top"));
    expect(document.querySelector<HTMLElement>(".tour-card")?.style.left).toBe("");
  });
});
