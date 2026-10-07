import { describe, expect, it } from "vitest";
import {
  classicPageFor,
  classicUrlFor,
  SCREENS,
  spaUrlFor,
  unportedClassicPage,
  withClassicBypass,
} from "./screens.ts";

/** `pattern` with its parameters filled with 7, 8, 9 in order. */
function sample(pattern: string): string {
  let next = 7;

  return pattern.replace(/:[a-z_]+/g, () => String(next++));
}

describe("the screen map", () => {
  it("maps ported classic pages to their SPA URLs", () => {
    expect(spaUrlFor("/")).toBe("/app/");
    expect(spaUrlFor("/rooms/12")).toBe("/app/r/12");
    expect(spaUrlFor("/rooms/12/@345")).toBe("/app/r/12/m/345");
    expect(spaUrlFor("/rooms/12/threads/9")).toBe("/app/r/12/t/9");
    expect(spaUrlFor("//rooms/12/")).toBe("/app/r/12");
  });

  it("matches only record ids, as the server does", () => {
    for (const path of [
      "/rooms/new",
      "/rooms/0",
      "/rooms/1e3",
      "/rooms/12.json",
      "/rooms/9007199254740992",
    ]) {
      expect(spaUrlFor(path), path).toBeNull();
    }

    expect(spaUrlFor("/rooms/9007199254740991")).toBe("/app/r/9007199254740991");
  });

  it("never sends people to a screen the SPA hasn't ported", () => {
    for (const screen of SCREENS.filter((row) => !row.ported)) {
      expect(spaUrlFor(sample(screen.classic)), screen.classic).toBeNull();
    }
  });

  it("maps every SPA URL back to its classic page", () => {
    for (const screen of SCREENS) {
      expect(classicUrlFor(sample(screen.spa)), screen.spa).toBe(sample(screen.classic));
    }

    expect(classicUrlFor("/app")).toBe("/");
    expect(classicUrlFor("/app/nowhere")).toBeNull();
    expect(classicUrlFor("/app/r/general")).toBeNull();
  });

  it("carries the query over without classic", () => {
    expect(spaUrlFor("/rooms/12", "?a=1&classic=0&b=2")).toBe("/app/r/12?a=1&b=2");
    expect(classicUrlFor("/app/search", "q=fire&classic=1")).toBe("/searches?q=fire");
    expect(withClassicBypass("/activity")).toBe("/activity?classic=1");
    expect(withClassicBypass("/searches?q=fire#top")).toBe("/searches?q=fire&classic=1#top");
  });

  it("finds the classic page for a router location, with or without the basepath", () => {
    expect(unportedClassicPage("/activity", "")).toBe("/activity?classic=1");
    expect(unportedClassicPage("/nowhere", "")).toBeNull();
    expect(classicPageFor("/r/12", "?message_id=9")).toBe("/rooms/12?message_id=9&classic=1");
    expect(classicPageFor("/app/r/12")).toBe("/rooms/12?classic=1");
    expect(classicPageFor("/")).toBe("/?classic=1");
    expect(classicPageFor("/nowhere")).toBe("/?classic=1");
  });
});
