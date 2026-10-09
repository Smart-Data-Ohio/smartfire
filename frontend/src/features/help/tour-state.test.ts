import { describe, expect, it, vi } from "vitest";
import { createTour } from "./tour-state.ts";
import { TOUR_STEPS } from "./tour-steps.ts";

function setup() {
  const stamp = vi.fn();
  const tour = createTour({ steps: TOUR_STEPS.length, stamp });

  return { tour, stamp, state: () => tour.store.getState() };
}

describe("product tour", () => {
  it("has classic's five steps in classic's order", () => {
    expect(TOUR_STEPS.map((step) => step.id)).toEqual([
      "sidebar",
      "composer",
      "huddles",
      "switcher",
      "shortcuts",
    ]);
  });

  it("starts by itself only for someone known not to have completed it", () => {
    const loading = setup();

    expect(loading.tour.autoStart(null)).toBe(false);
    expect(loading.state().open).toBe(false);

    const done = setup();

    expect(done.tour.autoStart(true)).toBe(false);
    expect(done.state().open).toBe(false);

    const fresh = setup();

    expect(fresh.tour.autoStart(false)).toBe(true);
    expect(fresh.state()).toEqual({ open: true, index: 0 });
  });

  it("starts by itself once per page load", () => {
    const { tour, state } = setup();

    tour.autoStart(false);
    tour.skip();

    expect(tour.autoStart(false)).toBe(false);
    expect(state().open).toBe(false);
  });

  it("steps forward and back within the steps", () => {
    const { tour, state, stamp } = setup();

    tour.start();
    tour.back();
    expect(state().index).toBe(0);

    tour.next();
    tour.next();
    expect(state().index).toBe(2);

    tour.back();
    expect(state().index).toBe(1);

    for (let step = 1; step < TOUR_STEPS.length - 1; step += 1) tour.next();

    expect(state()).toEqual({ open: true, index: TOUR_STEPS.length - 1 });
    expect(stamp).not.toHaveBeenCalled();
  });

  it("finishes from the last step's Next, stamping once", () => {
    const { tour, state, stamp } = setup();

    tour.start();

    for (const _ of TOUR_STEPS) tour.next();

    expect(state().open).toBe(false);
    expect(stamp).toHaveBeenCalledOnce();

    tour.next();
    tour.finish();
    tour.skip();
    expect(stamp).toHaveBeenCalledOnce();
  });

  it("stamps once when skipped, however many times it is dismissed", () => {
    const { tour, state, stamp } = setup();

    tour.start();
    tour.next();
    tour.skip();
    tour.skip();
    tour.finish();

    expect(state().open).toBe(false);
    expect(stamp).toHaveBeenCalledOnce();
  });

  it("restarts from the first step and stamps again when that run ends", () => {
    const { tour, state, stamp } = setup();

    tour.autoStart(false);
    tour.next();
    tour.skip();

    tour.start();
    expect(state()).toEqual({ open: true, index: 0 });

    tour.next();
    tour.next();
    tour.start();
    expect(state().index).toBe(0);

    tour.finish();
    expect(stamp).toHaveBeenCalledTimes(2);
  });
});
