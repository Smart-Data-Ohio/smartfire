import { createMemoryHistory } from "@tanstack/react-router";
import { describe, expect, it } from "vitest";
import { captureInitialMessageLink, messageLinkSnapshot } from "./message-link.ts";

describe("bare message link snapshots", () => {
  it("keeps repeated query keys and the original fragment", () => {
    expect(messageLinkSnapshot("/app/m/23?source=classic&source=link#top")).toEqual({
      messageId: 23,
      search: "?source=classic&source=link",
      hash: "#top",
    });
  });

  it.each([
    ["1e3", 1000],
    ["0x17", 23],
    ["%31", 1],
  ])("accepts the same id spelling %s as the bare message route", (segment, messageId) => {
    expect(messageLinkSnapshot(`/app/m/${segment}`)?.messageId).toBe(messageId);
  });

  it.each([
    "/app/r/4/m/23",
    "/app/m/23/boosts",
    "/app/people/23",
    "/app/m/0",
    "/app/m/-1",
    "/app/m/1.5",
    "/app/m/9007199254740992",
    "/app/m/%",
  ])("leaves the unrelated or invalid path %s alone", (path) => {
    expect(messageLinkSnapshot(path)).toBeNull();
  });

  it("preserves an original snapshot when reloading the canonicalized pending resolver", () => {
    const history = createMemoryHistory({
      initialEntries: ["/app/m/23?source=classic&source=link#top"],
    });

    captureInitialMessageLink(history);
    history.replace(
      "/app/m/23?source=%5B%22classic%22%2C%22link%22%5D#top",
      history.location.state,
    );
    captureInitialMessageLink(history);
    expect(history.location.state.smartfireMessageLink).toEqual({
      messageId: 23,
      search: "?source=classic&source=link",
      hash: "#top",
    });
    expect(history.length).toBe(1);
  });
});
