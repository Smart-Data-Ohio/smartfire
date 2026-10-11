import { afterEach, describe, expect, it } from "vitest";
import { movedRowFocus } from "./focus-targets.ts";

/** A section with its heading, and #general's row in its list (inert when folded). */
function section(key: string, rows: "live" | "folded" | "none"): HTMLElement {
  const holder = document.createElement("section");
  const trigger = document.createElement("button");
  const list = document.createElement("ul");

  holder.dataset.dropSection = key;
  trigger.className = "sidebar-section-trigger";
  holder.append(trigger, list);

  if (rows !== "none") {
    const row = document.createElement("a");

    row.dataset.dragHandle = "room-1";
    list.append(row);
    list.toggleAttribute("inert", rows === "folded");
  }

  return holder;
}

function sidebar(...sections: HTMLElement[]): HTMLElement {
  const container = document.createElement("div");

  container.append(...sections);
  document.body.append(container);

  return container;
}

const trigger = (container: HTMLElement, key: string) =>
  container.querySelector(`[data-drop-section="${key}"] .sidebar-section-trigger`);

afterEach(() => document.body.replaceChildren());

describe("focus after a row moves to another section", () => {
  it("goes to the row where it shows", () => {
    const container = sidebar(section("shared-1", "live"));

    expect(movedRowFocus(container, "room-1", "shared-1")).toBe(
      container.querySelector('[data-drag-handle="room-1"]'),
    );
  });

  it("goes to the heading of the folded section the row went into", () => {
    const container = sidebar(section("channels", "none"), section("shared-1", "folded"));

    expect(movedRowFocus(container, "room-1", null)).toBe(trigger(container, "shared-1"));
  });

  it("goes to the destination's heading when its folded list isn't drawn", () => {
    const container = sidebar(section("shared-1", "none"), section("shared-2", "none"));

    expect(movedRowFocus(container, "room-1", "shared-2")).toBe(trigger(container, "shared-2"));
    expect(movedRowFocus(container, "room-1", null)).toBeNull();
  });
});
