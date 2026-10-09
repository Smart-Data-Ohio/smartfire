import { describe, expect, it } from "vitest";
import { cardPosition, dockFor, MARGIN } from "./tour-placement.ts";

const VIEWPORT = { width: 1440, height: 900 };

const CARD = { width: 384, height: 200 };

describe("tour card placement", () => {
  it("puts the card beside a tall anchor, right when it fits", () => {
    const sidebar = { top: 0, left: 72, width: 260, height: 900 };

    expect(cardPosition(sidebar, CARD, VIEWPORT)).toEqual({ left: 344, top: MARGIN });
  });

  it("puts the card left of a tall anchor at the right edge", () => {
    const pane = { top: 0, left: 1200, width: 240, height: 900 };

    expect(cardPosition(pane, CARD, VIEWPORT)).toEqual({ left: 1200 - 384 - MARGIN, top: MARGIN });
  });

  it("hangs the card below a short anchor, or above it when below would overflow", () => {
    const button = { top: 10, left: 1300, width: 100, height: 32 };

    expect(cardPosition(button, CARD, VIEWPORT)).toEqual({
      left: VIEWPORT.width - CARD.width - MARGIN,
      top: 42 + MARGIN,
    });

    const composer = { top: 780, left: 340, width: 1000, height: 100 };

    expect(cardPosition(composer, CARD, VIEWPORT)).toEqual({ left: 340, top: 780 - 200 - MARGIN });
  });

  it("docks a phone's sheet away from its anchor", () => {
    expect(dockFor({ top: 8, left: 300, width: 44, height: 44 }, 740)).toBe("bottom");
    expect(dockFor({ top: 680, left: 0, width: 360, height: 60 }, 740)).toBe("top");
  });
});
