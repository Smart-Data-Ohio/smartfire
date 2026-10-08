import { describe, expect, it } from "vitest";
import { foreignThreadHref } from "./thread-target.ts";

describe("foreignThreadHref", () => {
  it("stays when the thread is in the URL's room", () => {
    expect(foreignThreadHref(4, { id: 9, roomId: 4 }, null)).toBeNull();
  });

  it("opens the thread in its own room, keeping a reply anchor", () => {
    expect(foreignThreadHref(4, { id: 9, roomId: 8 }, null)).toBe("/r/8/t/9");
    expect(foreignThreadHref(4, { id: 9, roomId: 8 }, 3)).toBe("/r/8/t/9?m=3");
  });
});
