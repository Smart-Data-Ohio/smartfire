import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Tabs } from "./tabs.tsx";

const ITEMS = [
  { value: "active", label: "Active" },
  { value: "closed", label: "Closed" },
];

describe("Tabs", () => {
  it("points the selected tab at its panel when there is one", () => {
    render(
      <Tabs items={ITEMS} value="active" onValueChange={() => undefined} label="Threads">
        <p>Panel</p>
      </Tabs>,
    );

    const tab = screen.getByRole("tab", { name: "Active" });
    const panel = screen.getByRole("tabpanel");

    expect(tab.getAttribute("aria-controls")).toBe(panel.id);
  });

  it("claims no panel when used as a filter without one", () => {
    render(<Tabs items={ITEMS} value="active" onValueChange={() => undefined} label="Threads" />);

    expect(screen.getByRole("tab", { name: "Active" }).hasAttribute("aria-controls")).toBe(false);
    expect(screen.queryByRole("tabpanel")).toBeNull();
  });
});
