import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { AppRoot } from "./app-root.tsx";

describe("AppRoot", () => {
  it("renders the app heading", () => {
    expect(renderToStaticMarkup(<AppRoot />)).toBe("<main><h1>Smartfire</h1></main>");
  });
});
