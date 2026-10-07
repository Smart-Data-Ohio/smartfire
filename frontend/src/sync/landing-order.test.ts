import { describe, expect, it } from "vitest";
import { landingOrder } from "./landing-order.ts";

interface Row {
  readonly id: number;
  readonly status: string;
}

const id = (row: Row) => row.id;

describe("landing order", () => {
  it("drops a slow load's record once a later write has landed it", () => {
    const order = landingOrder();
    const load = order.start();
    const ban = order.start();

    expect(order.claim(ban, [{ id: 7, status: "banned" }], id)).toHaveLength(1);
    expect(order.claim(load, [{ id: 7, status: "active" }], id)).toEqual([]);
  });

  it("keeps the records nobody newer has landed", () => {
    const order = landingOrder();
    const directory = order.start();
    const profile = order.start();

    order.claim(profile, [{ id: 7, status: "banned" }], id);

    const fresh = order.claim(
      directory,
      [
        { id: 7, status: "active" },
        { id: 8, status: "active" },
      ],
      id,
    );

    expect(fresh.map(id)).toEqual([8]);
  });

  it("lands replies that arrive in the order they started", () => {
    const order = landingOrder();
    const first = order.start();
    const second = order.start();

    expect(order.claim(first, [{ id: 7, status: "active" }], id)).toHaveLength(1);
    expect(order.claim(second, [{ id: 7, status: "banned" }], id)).toHaveLength(1);
  });
});
