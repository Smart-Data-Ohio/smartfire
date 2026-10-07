import { describe, expect, it } from "vitest";
import { messageDestination } from "./message-destination.ts";

describe("messageDestination", () => {
  it("opens a room message at its permalink and keeps the query and hash", () => {
    expect(
      messageDestination({ id: 23, roomId: 4, threadId: null }, "?filter=all&filter=mine", "top"),
    ).toBe("/app/r/4/m/23?filter=all&filter=mine#top");
  });

  it("opens a reply in its thread, replacing an old focus id and keeping other query values", () => {
    expect(messageDestination({ id: 23, roomId: 4, threadId: 7 }, "?m=9&filter=all")).toBe(
      "/app/r/4/t/7?m=23&filter=all",
    );
  });
});
