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
    expect(inPlaceTarget(link("/users/me/profile"), origin)).toBe("/app/settings");
    expect(inPlaceTarget(link("/activity"), origin)).toBe("/app/activity");
    expect(inPlaceTarget(link("/rooms/12/messages/34/fizzy_cards/new"), origin)).toBe(
      "/app/r/12/m/34/fizzy/new",
    );
    expect(inPlaceTarget(link("/rooms/12/threads/5/messages/34/fizzy_cards/new"), origin)).toBe(
      "/app/r/12/t/5/m/34/fizzy/new",
    );
  });

  it("leave everything else to the browser", () => {
    for (const anchor of [
      link("/rooms/12/events"),
      link("/users/7/profile"),
      link("/users/me/profile?classic=1#fizzy-connection-title"),
      link("/rooms/12?classic=1"),
      link("https://example.com/rooms/12"),
      link("/rooms/12", { target: "_blank" }),
      link("/rooms/12", { download: "" }),
    ]) {
      expect(inPlaceTarget(anchor, origin), anchor.outerHTML).toBeNull();
    }
  });
});
