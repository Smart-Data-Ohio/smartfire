import { expect, it } from "vitest";
import oracle from "../../vectors/unicode_casing_parity.json" with { type: "json" };
import { sidebarDirectLabel } from "./direct-label.ts";

it("orders DM names with the server's pinned Unicode casing", () => {
  expect(sidebarDirectLabel(null, ["ΟΣ", "οςa"])).toBe(oracle.sigma_names.join(", "));
});

it("orders supplementary code points after BMP names", () => {
  expect(sidebarDirectLabel(null, ["𐐀 Example", "Ａ Example"])).toBe("Ａ Example, 𐐀 Example");
});
