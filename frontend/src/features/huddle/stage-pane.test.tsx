import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { StageAudienceStatus } from "./stage-pane.tsx";

afterEach(cleanup);

it("shows the declared budget and open availability", () => {
  render(
    <StageAudienceStatus
      stage={{ audienceCount: 12, audienceBudget: 300, audienceAvailability: "open" }}
    />,
  );
  expect(screen.getByRole("status").textContent).toBe("12 / 300 listening · Open");
});

it("shows full availability including reservations and a lowered budget", () => {
  const view = render(
    <StageAudienceStatus
      stage={{ audienceCount: 0, audienceBudget: 1, audienceAvailability: "full" }}
    />,
  );

  expect(screen.getByRole("status").textContent).toBe("0 / 1 listening · Full");
  view.rerender(
    <StageAudienceStatus
      stage={{ audienceCount: 2, audienceBudget: 1, audienceAvailability: "full" }}
    />,
  );
  expect(screen.getByRole("status").textContent).toBe("2 / 1 listening · Full");
});
