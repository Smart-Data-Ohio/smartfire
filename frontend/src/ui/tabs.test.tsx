import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Tabs, tabId } from "./tabs.tsx";

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

  it("points the selected tab at a panel laid out elsewhere", () => {
    render(
      <>
        <Tabs
          id="saved-filter"
          panelId="saved-panel"
          items={ITEMS}
          value="closed"
          onValueChange={() => undefined}
          label="Show"
        />
        <div role="tabpanel" id="saved-panel" aria-labelledby={tabId("saved-filter", "closed")}>
          Rows
        </div>
      </>,
    );

    const tab = screen.getByRole("tab", { name: "Closed" });

    expect(tab.getAttribute("aria-controls")).toBe("saved-panel");
    expect(screen.getByRole("tab", { name: "Active" }).hasAttribute("aria-controls")).toBe(false);
    expect(screen.getByRole("tabpanel", { name: "Closed" })).toBeTruthy();
  });

  it("claims no panel when used as a filter without one", () => {
    render(<Tabs items={ITEMS} value="active" onValueChange={() => undefined} label="Threads" />);

    expect(screen.getByRole("tab", { name: "Active" }).hasAttribute("aria-controls")).toBe(false);
    expect(screen.queryByRole("tabpanel")).toBeNull();
  });
});
