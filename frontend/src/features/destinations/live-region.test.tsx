import { act, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { useAnnouncer } from "./live-region.tsx";

let say: (text: string) => void = () => {};

function Page() {
  const { announce, region } = useAnnouncer();

  say = announce;

  return region;
}

describe("the page's live region", () => {
  it("says each message politely, a repeat as a new node", () => {
    render(<Page />);

    const region = screen.getByRole("status");

    expect(region.getAttribute("aria-live")).toBe("polite");
    expect(region.textContent).toBe("");

    act(() => say("Marked handled"));

    const first = region.firstElementChild;

    expect(region.textContent).toBe("Marked handled");

    act(() => say("Marked handled"));

    expect(region.textContent).toBe("Marked handled");
    expect(region.firstElementChild).not.toBe(first);
  });

  it("can interrupt instead, and still repeats", () => {
    function Urgent() {
      const { announce, region } = useAnnouncer("assertive");

      say = announce;

      return region;
    }

    const { container } = render(<Urgent />);
    const region = container.querySelector('[aria-live="assertive"]');

    expect(region?.getAttribute("role")).toBeNull();

    act(() => say("Team"));

    const first = region?.firstElementChild;

    act(() => say("Team"));

    expect(region?.textContent).toBe("Team");
    expect(region?.firstElementChild).not.toBe(first);
  });
});
