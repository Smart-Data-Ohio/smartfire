import { expect, it } from "vitest";
import oracle from "../../vectors/unicode_casing_parity.json" with { type: "json" };
import { directRoomLabel, sidebarDirectLabel } from "./direct-label.ts";

it("orders DM names with the server's pinned Unicode casing", () => {
  expect(sidebarDirectLabel(null, ["ΟΣ", "οςa"])).toBe(oracle.sigma_names.join(", "));
});

it("orders supplementary code points after BMP names", () => {
  expect(sidebarDirectLabel(null, ["𐐀 Example", "Ａ Example"])).toBe("Ａ Example, 𐐀 Example");
});

it("keeps first names in non-sidebar DM labels", () => {
  expect(directRoomLabel(null, ["Maya Okafor"])).toBe("Maya");
  expect(
    directRoomLabel(null, [
      "Theo Nakamura",
      "Sam Whitfield",
      "Priya Raman",
      "Maya Okafor",
      "Jonah Lindqvist",
    ]),
  ).toBe("Jonah, Maya, Priya +2");
  expect(directRoomLabel("Launch crew", ["Maya Okafor"])).toBe("Launch crew");
});
