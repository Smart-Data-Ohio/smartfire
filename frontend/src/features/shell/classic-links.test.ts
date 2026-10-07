import { describe, expect, it } from "vitest";
import { inPlaceTarget } from "./classic-links.ts";

function link(href: string, attributes: Record<string, string> = {}): HTMLAnchorElement {
  const anchor = document.createElement("a");

  anchor.href = href;

  for (const [name, value] of Object.entries(attributes)) {
    anchor.setAttribute(name, value);
  }

  return anchor;
}

const origin = window.location.origin;

describe("links to classic pages", () => {
  it("open ported pages in place", () => {
    expect(inPlaceTarget(link("/rooms/12/@34"), origin)).toBe("/app/r/12/m/34");
    expect(inPlaceTarget(link(`${origin}/rooms/12/threads/5?m=8#x`), origin)).toBe(
      "/app/r/12/t/5?m=8#x",
    );
    expect(inPlaceTarget(link("/activity"), origin)).toBe("/app/activity");
    expect(inPlaceTarget(link("/work?state=done"), origin)).toBe("/app/work?state=done");
  });

  it("leave everything else to the browser", () => {
    for (const anchor of [
      link("/rooms/12/events/3"),
      link("/users/me/profile"),
      link("/rooms/12?classic=1"),
      link("https://example.com/rooms/12"),
      link("/rooms/12", { target: "_blank" }),
      link("/rooms/12", { download: "" }),
    ]) {
      expect(inPlaceTarget(anchor, origin), anchor.outerHTML).toBeNull();
    }
  });
});
