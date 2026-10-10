import { describe, expect, it } from "vitest";
import { SCREENS, spaUrlFor } from "./screens.ts";

/** `pattern` with its parameters filled with 7, 8, 9 in order. */
function sample(pattern: string): string {
  let next = 7;

  return pattern.replace(/:[a-z_]+/g, () => String(next++));
}

describe("the screen map", () => {
  it("maps profile and edit aliases, with the viewer's numeric id opening settings", () => {
    for (const path of ["/users/7", "/users/7/profile", "/users/7/profile/edit"]) {
      expect(spaUrlFor(path, "?source=profile&classic=0", 7)).toBe("/app/settings?source=profile");
      expect(spaUrlFor(path, "", 8)).toBe("/app/people/7");
    }

    expect(spaUrlFor("/users/me/profile/edit")).toBe("/app/settings");
  });
  it("maps ported classic pages to their SPA URLs", () => {
    expect(spaUrlFor("/")).toBe("/app/");
    expect(spaUrlFor("/rooms/12")).toBe("/app/r/12");
    expect(spaUrlFor("/rooms/boards/12/edit")).toBe("/app/r/12/settings");
    expect(spaUrlFor("/rooms/boards/12/automations")).toBe("/app/r/12/automations");
    expect(spaUrlFor("/rooms/12/@345")).toBe("/app/r/12/m/345");
    expect(spaUrlFor("/rooms/12/threads/9")).toBe("/app/r/12/t/9");
    expect(spaUrlFor("//rooms/12/")).toBe("/app/r/12");
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

  it("translates room notification queries into the thread and message routes", () => {
    expect(spaUrlFor("/rooms/12", "?thread=9&message_id=4")).toBe("/app/r/12/t/9?m=4");
    expect(spaUrlFor("/rooms/12", "?thread=9")).toBe("/app/r/12/t/9");
    expect(spaUrlFor("/rooms/12", "?message_id=4")).toBe("/app/r/12/m/4");
    expect(spaUrlFor("/rooms/12", "?x=1&thread=9&classic=1&y=2")).toBe("/app/r/12/t/9?x=1&y=2");
    expect(spaUrlFor("/rooms/12", "?thread=nope&x=1")).toBe("/app/r/12?thread=nope&x=1");
  });

  it("sends a thread's content message_id to the SPA's reply parameter m", () => {
    const content = "/rooms/12/threads/9/content";

    expect(spaUrlFor(content, "?message_id=123")).toBe("/app/r/12/t/9?m=123");
    expect(spaUrlFor(content, "?message_id=abc")).toBe("/app/r/12/t/9");
    expect(spaUrlFor(content, "?x=1&classic=1")).toBe("/app/r/12/t/9?x=1");
    expect(spaUrlFor(content)).toBe("/app/r/12/t/9");
  });

  it("decodes room notification ids and uses the classic duplicate", () => {
    expect(spaUrlFor("/rooms/12", "?thread=%39")).toBe("/app/r/12/t/9");
    expect(spaUrlFor("/rooms/12", "?message_id=%34")).toBe("/app/r/12/m/4");
    expect(spaUrlFor("/rooms/12", "?th%72ead=%39")).toBe("/app/r/12/t/9");
    expect(spaUrlFor("/rooms/12", "?thread=9&thread=8")).toBe("/app/r/12/t/9");
    expect(spaUrlFor("/rooms/12", "?thread=%39&thread=8")).toBe("/app/r/12/t/9");
    expect(spaUrlFor("/rooms/12", "?thread=nope&thread=9")).toBe("/app/r/12?thread=nope&thread=9");
    expect(spaUrlFor("/rooms/12", "?message_id=4&message_id=5")).toBe("/app/r/12/m/5");
    expect(spaUrlFor("/rooms/12", "?message_id=4&message_id=nope")).toBe(
      "/app/r/12?message_id=4&message_id=nope",
    );
    expect(spaUrlFor("/rooms/12", "?thread=9&thread=8&message_id=4&message_id=5")).toBe(
      "/app/r/12/t/9?m=4",
    );
    expect(spaUrlFor("/rooms/12", "?thread=9&message_id=nope&message_id=5")).toBe("/app/r/12/t/9");
    expect(spaUrlFor("/rooms/12", "?message_id=nope&message_id=5")).toBe("/app/r/12/m/5");
    // `%ZZ` is not an id. HTTP rejects that escape with 400 before routing; this only pins the translator.
    expect(spaUrlFor("/rooms/12", "?thread=%ZZ&x=1")).toBe("/app/r/12?thread=%ZZ&x=1");
  });

  it("carries the query over without classic", () => {
    expect(spaUrlFor("/rooms/12", "?a=1&classic=0&b=2")).toBe("/app/r/12?a=1&b=2");
    expect(spaUrlFor("/searches", "q=fire&classic=1")).toBe("/app/search?q=fire");
  });
});
