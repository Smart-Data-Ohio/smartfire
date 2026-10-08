import { describe, expect, it } from "vitest";
import {
  classicPageFor,
  classicToSpaUrl,
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

/** Parameter names do not distinguish paths that map to the same SPA screen. */
function screenPattern(pattern: string): string {
  return pattern.replace(/:[a-z_]+/g, ":id");
}

describe("the screen map", () => {
  it("maps ported classic pages to their SPA URLs", () => {
    expect(spaUrlFor("/")).toBe("/app/");
    expect(spaUrlFor("/rooms/12")).toBe("/app/r/12");
    expect(spaUrlFor("/rooms/12/@345")).toBe("/app/r/12/m/345");
    expect(spaUrlFor("/rooms/12/threads/9")).toBe("/app/r/12/t/9");
    expect(spaUrlFor("//rooms/12/")).toBe("/app/r/12");
  });

  it("maps notification rooms, permalinks, threads and settings through the screen map", () => {
    const origin = "https://smartfire.example";

    expect(classicToSpaUrl("/rooms/12", origin)).toBe("/app/r/12");
    expect(classicToSpaUrl("/rooms/12/@345", origin)).toBe("/app/r/12/m/345");
    expect(classicToSpaUrl("/rooms/12/threads/9", origin)).toBe("/app/r/12/t/9");
    expect(classicToSpaUrl("/users/me/profile", origin)).toBe("/app/settings");
    expect(classicToSpaUrl(`${origin}/users/me/sessions`, origin)).toBe("/app/settings/sessions");
  });

  it("preserves a notification's query and fragment without rewriting their encoding", () => {
    expect(
      classicToSpaUrl(
        "/rooms/12/threads/9?m=4&label=a%20b&classic=1&label=c+d#reply",
        "https://smartfire.example",
      ),
    ).toBe("/app/r/12/t/9?m=4&label=a%20b&classic=1&label=c+d#reply");
  });

  it("opens a person's notification path on their SPA page", () => {
    expect(classicToSpaUrl("/users/7", "https://smartfire.example")).toBe("/app/people/7");
  });

  it("opens the work page's notification path on the SPA's, keeping its filter", () => {
    expect(classicToSpaUrl("/work?status=open", "https://smartfire.example")).toBe(
      "/app/work?status=open",
    );
  });

  it("leaves unknown notification paths to classic", () => {
    expect(classicToSpaUrl("/nowhere", "https://smartfire.example")).toBeNull();
  });

  it("never maps a foreign origin or malformed notification URL", () => {
    const origin = "https://smartfire.example";

    expect(classicToSpaUrl("https://foreign.example/rooms/12", origin)).toBeNull();
    expect(classicToSpaUrl("//foreign.example/rooms/12", origin)).toBeNull();
    expect(classicToSpaUrl("http://[", origin)).toBeNull();
    expect(classicToSpaUrl("/rooms/12", "invalid origin")).toBeNull();
  });

  it.each([
    ["/rooms/12/messages/345", "/app/r/12/m/345"],
    ["/rooms/12/messages/345/edit", "/app/r/12/m/345"],
    ["/messages/345", "/app/m/345"],
    ["/messages/345/edit", "/app/m/345"],
    ["/messages/345/boosts", "/app/m/345"],
    ["/messages/345/boosts/new", "/app/m/345"],
    ["/rooms/12/threads", "/app/r/12/threads"],
    ["/rooms/12/files", "/app/r/12/files"],
    ["/rooms/12/pins", "/app/r/12/pins"],
    ["/rooms/12/involvement", "/app/r/12/notifications"],
  ])("maps the existing-screen classic link %s to %s", (classic, spa) => {
    expect(spaUrlFor(classic, "?source=classic")).toBe(`${spa}?source=classic`);
  });

  it("opens every ported classic path, including aliases of an existing destination", () => {
    for (const screen of SCREENS.filter((row) => row.ported)) {
      const canonical = SCREENS.find((row) => row.ported && row.classic === screen.classic);

      expect(spaUrlFor(sample(screen.classic)), screen.classic).toBe(
        canonical === undefined ? null : sample(canonical.spa),
      );
    }
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

  it("maps every SPA URL back to its first classic page, including shared destination aliases", () => {
    for (const screen of SCREENS) {
      const canonical = SCREENS.find((row) => screenPattern(row.spa) === screenPattern(screen.spa));

      expect(classicUrlFor(sample(screen.spa)), screen.spa).toBe(
        canonical === undefined ? null : sample(canonical.classic),
      );
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
