import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { Button } from "./button.tsx";

describe("Button", () => {
  it("is busy while loading: keeps its name, ignores clicks, and stays focusable", async () => {
    const user = userEvent.setup();
    let clicks = 0;

    const onClick = () => {
      clicks += 1;
    };

    const { rerender } = render(
      <Button loading loadingLabel="Saving" onClick={onClick}>
        Save
      </Button>,
    );

    const button = screen.getByRole("button", { name: "Save" });

    expect(button.getAttribute("aria-busy")).toBe("true");
    expect(button.getAttribute("aria-disabled")).toBe("true");
    expect(button.hasAttribute("disabled")).toBe(false);

    await user.click(button);
    expect(clicks).toBe(0);

    // aria-disabled rather than disabled, so focus stays put and screen readers hear "busy".
    expect(document.activeElement).toBe(button);
    await user.keyboard("{Enter}");
    expect(clicks).toBe(0);

    rerender(
      <Button loading={false} loadingLabel="Saving" onClick={onClick}>
        Save
      </Button>,
    );

    expect(button.hasAttribute("aria-busy")).toBe(false);
    await user.click(button);
    expect(clicks).toBe(1);
  });

  it("locks its width while loading by stacking the label and the busy state", () => {
    render(
      <Button loading loadingLabel="Saving">
        Save
      </Button>,
    );

    const button = screen.getByRole("button", { name: "Save" });
    const label = button.querySelector(".button-label");
    const busy = button.querySelector(".button-busy");

    // Both stay in the DOM in one grid cell; only their visibility swaps.
    expect(label?.classList.contains("is-exit")).toBe(true);
    expect(busy?.classList.contains("is-exit")).toBe(false);
    expect(busy?.textContent).toContain("Saving");
  });

  it("does nothing when disabled", async () => {
    const user = userEvent.setup();
    let clicks = 0;

    render(
      <Button
        disabled
        onClick={() => {
          clicks += 1;
        }}
      >
        Send
      </Button>,
    );

    await user.click(screen.getByRole("button", { name: "Send" }));
    expect(clicks).toBe(0);
  });
});
