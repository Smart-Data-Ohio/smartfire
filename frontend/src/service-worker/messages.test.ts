import { describe, expect, it } from "vitest";
import { PAGE_BUILD, pageBuildMessage, REQUEST_PAGE_BUILD, requestsPageBuild } from "./messages.ts";

const origin = "https://smartfire.test";

describe("page build messages", () => {
  it("uses the actual entry URL, preserving its complete identity", () => {
    const page = `${origin}/app/assets/index-abCD1234.js?build=b`;

    expect(pageBuildMessage({ data: { kind: PAGE_BUILD, page } }, origin)).toBe(page);
    expect(requestsPageBuild({ data: { kind: REQUEST_PAGE_BUILD } })).toBe(true);
  });

  it("rejects malformed messages, foreign origins and private or unhashed URLs", () => {
    for (const value of [
      null,
      "build",
      {},
      { kind: PAGE_BUILD, page: 12 },
      { kind: PAGE_BUILD, page: { toString: 12 } },
      { kind: PAGE_BUILD, page: "/app/assets/index-abCD1234.js" },
      { kind: PAGE_BUILD, page: "https://elsewhere.test/app/assets/index-abCD1234.js" },
      { kind: PAGE_BUILD, page: `${origin}/api/v1/boot` },
      { kind: PAGE_BUILD, page: `${origin}/app/assets/index.js` },
      { kind: "another-message", page: `${origin}/app/assets/index-abCD1234.js` },
    ]) {
      expect(pageBuildMessage({ data: value }, origin)).toBeNull();
      expect(requestsPageBuild({ data: value })).toBe(false);
    }
  });
});
